package com.shininggrimace.syncpak

import com.android.billingclient.api.Purchase
import org.junit.Assert.*
import org.junit.Test

class SupportPurchasesTest {
    private fun purchase(product: String = "coffee", pending: Boolean = false, acknowledged: Boolean = false) =
        Purchase("""{"productId":"$product","purchaseToken":"test-token","purchaseState":${if (pending) 4 else 0},"acknowledged":$acknowledged}""", "")

    @Test fun pendingPaymentCannotBecomeCompletedSupport() {
        val purchases = SupportPurchases("coffee", listOf(purchase(pending = true)))
        assertTrue(purchases.pending)
        assertNull(purchases.completed)
    }

    @Test fun unrelatedPurchasedProductsDoNotHideTheCoffeeButton() {
        val purchases = SupportPurchases("coffee", listOf(purchase("another-product")))
        assertFalse(purchases.pending)
        assertNull(purchases.completed)
    }

    @Test fun restoredAcknowledgedPurchaseIsRecognizedWithoutBuyingAgain() {
        val purchases = SupportPurchases("coffee", listOf(purchase(acknowledged = true)))
        assertNotNull(purchases.completed)
        assertTrue(purchases.completed!!.isAcknowledged)
    }

    @Test fun unacknowledgedCompletedPurchaseIsAvailableForAcknowledgementRetry() {
        val purchases = SupportPurchases("coffee", listOf(purchase()))
        assertNotNull(purchases.completed)
        assertFalse(purchases.completed!!.isAcknowledged)
    }

    @Test fun completedSupportTakesPriorityOverAnotherPendingTransaction() {
        val purchases = SupportPurchases("coffee", listOf(purchase(pending = true), purchase()))
        assertNotNull(purchases.completed)
    }

    @Test fun lateFailuresCannotReplaceThePermanentThankYou() {
        val owned = SupportState(SupportState.OWNED)
        for (phase in listOf(SupportState.LOADING, SupportState.READY, SupportState.PENDING, SupportState.ERROR)) {
            assertEquals(owned, owned.transition(SupportState(phase)))
        }
    }

    @Test fun pendingAndFailedTransactionsCanBeRestoredOnRetry() {
        for (phase in listOf(SupportState.PENDING, SupportState.ERROR)) {
            assertEquals(SupportState.OWNED,
                SupportState(phase).transition(SupportState(SupportState.OWNED)).phase)
        }
    }
}
