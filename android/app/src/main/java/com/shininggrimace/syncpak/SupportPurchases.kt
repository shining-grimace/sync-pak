package com.shininggrimace.syncpak

import com.android.billingclient.api.Purchase

/** Ignores other products and never treats a pending payment as completed support. */
internal class SupportPurchases(productId: String, purchases: List<Purchase>) {
    private val matching = purchases.filter { productId in it.products }
    val completed = matching.firstOrNull { it.purchaseState == Purchase.PurchaseState.PURCHASED }
    val pending = matching.any { it.purchaseState == Purchase.PurchaseState.PENDING }
}
