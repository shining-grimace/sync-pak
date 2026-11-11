package com.shininggrimace.syncpak

import android.app.Activity
import android.content.Context
import com.android.billingclient.api.*

/** Activity-owned Play connection; callbacks and UI state are serialized on the main thread. */
internal class SupportBilling(
    private val activity: Activity,
    private val changed: (SupportState) -> Unit,
) : PurchasesUpdatedListener, BillingClientStateListener {
    private val productId = BuildConfig.BUY_ME_A_COFFEEE_PRODUCT_ID
    private val preferences = activity.getSharedPreferences("support-purchase", Context.MODE_PRIVATE)
    private var state = if (preferences.getBoolean(productId, false)) SupportState(SupportState.OWNED)
        else SupportState()
    private var destroyed = false
    private var connecting = false
    private var querying = false
    private var revision = 0
    private var details: ProductDetails? = null
    private var offerToken: String? = null
    private val acknowledging = mutableSetOf<String>()
    private val client = BillingClient.newBuilder(activity)
        .setListener(this)
        .enablePendingPurchases(PendingPurchasesParams.newBuilder().enableOneTimeProducts().build())
        .enableAutoServiceReconnection()
        .build()

    fun snapshot() = changed(state)

    private fun publish(next: SupportState) {
        if (destroyed) return
        state = state.transition(next)
        changed(state)
    }

    private fun main(action: () -> Unit) = activity.runOnUiThread { if (!destroyed) action() }

    fun refresh() {
        if (destroyed || connecting || querying || state.phase == SupportState.OWNED) return
        if (productId.startsWith("TODO_") || productId.isBlank()) {
            publish(SupportState(SupportState.UNCONFIGURED, message = "Support purchases are not available yet."))
            return
        }
        if (acknowledging.isNotEmpty()) return
        if (!client.isReady) {
            connecting = true
            publish(SupportState())
            client.startConnection(this)
        } else queryOwnership()
    }

    override fun onBillingSetupFinished(result: BillingResult) = main {
        connecting = false
        if (result.responseCode == BillingClient.BillingResponseCode.OK) queryOwnership()
        else error("Google Play billing is unavailable. Please try again.")
    }

    override fun onBillingServiceDisconnected() = main {
        connecting = false
        querying = false
        revision++
        if (state.phase != SupportState.PROCESSING && state.phase != SupportState.PENDING)
            error("Google Play disconnected. Please try again.")
    }

    private fun queryOwnership() {
        querying = true
        val expected = ++revision
        if (state.phase != SupportState.PENDING && state.phase != SupportState.PROCESSING)
            publish(SupportState())
        client.queryPurchasesAsync(QueryPurchasesParams.newBuilder()
            .setProductType(BillingClient.ProductType.INAPP).build()) { result, purchases -> main {
            if (expected != revision) return@main
            querying = false
            if (result.responseCode != BillingClient.BillingResponseCode.OK) {
                error("Could not check your purchase with Google Play. Please try again.")
            } else {
                val support = SupportPurchases(productId, purchases)
                val completed = support.completed
                when {
                    completed != null -> complete(completed)
                    support.pending -> publish(SupportState(SupportState.PENDING))
                    else -> loadProduct()
                }
            }
        } }
    }

    private fun loadProduct() {
        querying = true
        val expected = revision
        publish(SupportState())
        details = null
        offerToken = null
        val product = QueryProductDetailsParams.Product.newBuilder()
            .setProductId(productId).setProductType(BillingClient.ProductType.INAPP).build()
        client.queryProductDetailsAsync(QueryProductDetailsParams.newBuilder()
            .setProductList(listOf(product)).build()) { result, products -> main {
            if (expected != revision) return@main
            querying = false
            val found = products.productDetailsList.firstOrNull { it.productId == productId }
            val offer = found?.oneTimePurchaseOfferDetailsList?.firstOrNull {
                it.purchaseOptionId != null && it.rentalDetails == null && it.preorderDetails == null
            }
            if (result.responseCode != BillingClient.BillingResponseCode.OK || found == null || offer == null) {
                error("This support purchase is currently unavailable. Please try again later.")
            } else if (state.phase == SupportState.LOADING) {
                details = found
                offerToken = offer.offerToken
                publish(SupportState(SupportState.READY, offer.formattedPrice))
            }
        } }
    }

    fun buy() {
        if (destroyed || state.phase != SupportState.READY) return
        val product = details ?: return
        val token = offerToken ?: return
        publish(SupportState(SupportState.PROCESSING))
        val params = BillingFlowParams.ProductDetailsParams.newBuilder()
            .setProductDetails(product).setOfferToken(token).build()
        val result = client.launchBillingFlow(activity, BillingFlowParams.newBuilder()
            .setProductDetailsParamsList(listOf(params)).build())
        if (result.responseCode != BillingClient.BillingResponseCode.OK) handleFailure(result)
    }

    override fun onPurchasesUpdated(result: BillingResult, purchases: List<Purchase>?) = main {
        revision++
        querying = false
        if (result.responseCode != BillingClient.BillingResponseCode.OK) handleFailure(result)
        else {
            val support = SupportPurchases(productId, purchases.orEmpty())
            val completed = support.completed
            when {
                completed != null -> complete(completed)
                support.pending ->
                    publish(SupportState(SupportState.PENDING))
                else -> { publish(SupportState()); queryOwnership() }
            }
        }
    }

    private fun complete(purchase: Purchase) {
        if (purchase.isAcknowledged) { remember(); return }
        publish(SupportState(SupportState.PROCESSING))
        if (!acknowledging.add(purchase.purchaseToken)) return
        client.acknowledgePurchase(AcknowledgePurchaseParams.newBuilder()
            .setPurchaseToken(purchase.purchaseToken).build()) { result -> main {
            acknowledging.remove(purchase.purchaseToken)
            if (result.responseCode == BillingClient.BillingResponseCode.OK) remember()
            else error("Payment received. Reopen Settings or tap Retry to finish confirming your support.")
        } }
    }

    private fun remember() {
        preferences.edit().putBoolean(productId, true).apply()
        publish(SupportState(SupportState.OWNED))
    }

    private fun handleFailure(result: BillingResult) {
        when (result.responseCode) {
            BillingClient.BillingResponseCode.USER_CANCELED -> { publish(SupportState()); queryOwnership() }
            BillingClient.BillingResponseCode.ITEM_ALREADY_OWNED -> { publish(SupportState()); queryOwnership() }
            else -> error("Google Play could not complete the purchase. Please try again.")
        }
    }

    private fun error(message: String) = publish(SupportState(SupportState.ERROR, message = message))

    fun destroy() {
        destroyed = true
        client.endConnection()
    }
}
