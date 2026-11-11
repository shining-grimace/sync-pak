# Android support purchase

Settings shows an Android-only **Support SyncPak** section below About. It uses
Google Play Billing Library 9.1.0, the latest release listed in Google's
[release notes](https://developer.android.com/google/play/billing/release-notes)
when this integration was added.

## Configure the product

Replace `BUY_ME_A_COFFEEE_PRODUCT_ID` in `android/app/build.gradle.kts` with the
product ID from Play Console. Create a one-time product with a **buy** purchase
option, activate it in the intended regions, and set its localized prices.
Use a normal purchase option, not rental or pre-order. The UI gets its formatted
price from the same offer token passed to Google's purchase sheet.

Until the placeholder is replaced, the section says support purchases are not
available yet and purchasing is disabled. The placeholder is never sent to Play.

This is a **non-consumable** product. The app acknowledges completed purchases
and never consumes them. It grants no features and does not remove advertising.
Do not change the product ID after release: ownership and the local thank-you
record are keyed by it.

## Ownership and recovery

Only `PURCHASED` transactions for this product can complete support. Pending
payments show a pending state without acknowledgement or a thank-you. The app
queries owned purchases on connection, resume, Retry, and opening Settings. It
acknowledges unacknowledged purchases, including purchases recovered after an
interruption, before showing the permanent thank-you. Acknowledgement errors
offer Retry rather than presenting another purchase action.

Once acknowledged, ownership is saved in Android private preferences and the
button is permanently replaced by the thank-you message on that installation,
including offline. This record is deliberately not cleared by an empty purchase
query or an account switch. Reinstallation or clearing app data removes the local
record; Google Play restores the non-consumable purchase for the owning account.
Refunded/revoked purchases might not restore after clearing data or reinstalling.

This is a client-only billing integration: Play handles payment and returns
purchase state; the app acknowledges through BillingClient. It does not send
purchase tokens to SyncPak servers, configuration exports, diagnostics, or logs.
There is no server receipt verification or server refund processing.

## Validation

```sh
./android/gradlew --project-dir android :app:testDebugUnitTest :app:compileReleaseKotlin
./android/gradlew --project-dir android :app:buildRustDebugArm64
```

After configuring the real product, use a Play internal-test release and a
license tester to verify on a device:

- Localized price and the final Play purchase sheet agree.
- Cancellation restores the purchase action without an error message.
- Successful payment is acknowledged and the button becomes a thank-you.
- Restarting offline still shows the thank-you.
- Clearing app data or reinstalling restores the purchase with the same Play account.
- Delayed test payments remain pending; completion restores and acknowledges them.
- A cancelled pending payment returns to an available purchase after refresh.
- Interrupt the app during payment and acknowledgement, then resume or Retry.
- Repeated taps and late callbacks cannot launch duplicate purchases or replace a thank-you.
- Non-Play/unavailable billing shows an explanation and Retry.
- Desktop Settings has no support-purchase section.

See Google's [integration guide](https://developer.android.com/google/play/billing/integrate)
and [billing testing guide](https://developer.android.com/google/play/billing/test).
