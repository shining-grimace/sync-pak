package com.shininggrimace.syncpak

import android.app.Activity
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.util.Log
import android.view.Gravity
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.PopupWindow
import android.widget.TextView
import com.google.android.libraries.ads.mobile.sdk.MobileAds
import com.google.android.libraries.ads.mobile.sdk.banner.AdSize
import com.google.android.libraries.ads.mobile.sdk.banner.AdView
import com.google.android.libraries.ads.mobile.sdk.banner.BannerAd
import com.google.android.libraries.ads.mobile.sdk.banner.BannerAdRequest
import com.google.android.libraries.ads.mobile.sdk.common.AdLoadCallback
import com.google.android.libraries.ads.mobile.sdk.common.LoadAdError
import com.google.android.libraries.ads.mobile.sdk.common.RequestConfiguration
import com.google.android.libraries.ads.mobile.sdk.initialization.InitializationConfig
import com.google.android.ump.ConsentInformation
import com.google.android.ump.ConsentDebugSettings
import com.google.android.ump.ConsentRequestParameters
import com.google.android.ump.UserMessagingPlatform
import kotlin.concurrent.thread
import kotlin.math.roundToInt

/** Keeps advertising native, consent-first, and separate from the Slint UI. */
class AdsController(private val activity: Activity) {
    private val consentInformation = UserMessagingPlatform.getConsentInformation(activity)
    private var consentRequested = false
    private var consentRequestInProgress = false
    private var mobileAdsInitialized = false
    private var mobileAdsInitializationInProgress = false
    private var placement = Placement.HIDDEN
    private var banner: AdView? = null
    private var advertisingContainer: LinearLayout? = null
    private var advertisingPopup: PopupWindow? = null
    private var resumed = false
    private var destroyed = false

    fun resume() {
        resumed = true
        renderPlacement()
    }

    fun pause() {
        resumed = false
        removeAdvertising()
    }

    fun windowAvailable() {
        renderPlacement()
    }

    private fun canShow(): Boolean = !destroyed && resumed &&
        !activity.isFinishing && !activity.isDestroyed &&
        activity.window.decorView.isAttachedToWindow && activity.window.decorView.windowToken != null

    val privacyChoicesAvailable: Boolean
        get() = consentInformation.privacyOptionsRequirementStatus ==
            ConsentInformation.PrivacyOptionsRequirementStatus.REQUIRED

    fun start() {
        if (destroyed || consentRequested || consentRequestInProgress) {
            return
        }
        consentRequestInProgress = true
        Log.i(LOG_TAG, "Refreshing advertising consent information")
        val parameters = consentRequestParameters()
        consentInformation.requestConsentInfoUpdate(
            activity,
            parameters,
            {
                if (destroyed) return@requestConsentInfoUpdate
                consentRequestInProgress = false
                consentRequested = true
                notifyPrivacyChoicesAvailability()
                UserMessagingPlatform.loadAndShowConsentFormIfRequired(activity) { formError ->
                    if (destroyed) return@loadAndShowConsentFormIfRequired
                    notifyPrivacyChoicesAvailability()
                    if (formError == null) {
                        Log.i(LOG_TAG, "Advertising consent form completed or was not required")
                        initializeAdsIfPermitted()
                    } else {
                        Log.w(LOG_TAG, "Advertising consent form could not be shown: ${formError.message}")
                    }
                }
            },
            { error ->
                if (destroyed) return@requestConsentInfoUpdate
                consentRequestInProgress = false
                // Do not initialize or request ads if a current consent decision cannot be obtained.
                Log.w(LOG_TAG, "Advertising consent information could not be refreshed: ${error.message}")
                notifyPrivacyChoicesAvailability()
            },
        )
    }

    fun setPlacement(value: Int) {
        placement = Placement.from(value)
        renderPlacement()
    }

    fun showPrivacyChoices() {
        if (!canShow() || !privacyChoicesAvailable) {
            return
        }
        UserMessagingPlatform.showPrivacyOptionsForm(activity) {
            if (destroyed) return@showPrivacyOptionsForm
            notifyPrivacyChoicesAvailability()
            renderPlacement()
        }
    }

    fun destroy() {
        destroyed = true
        resumed = false
        removeAdvertising()
    }

    private fun initializeAdsIfPermitted() {
        if (destroyed || !consentInformation.canRequestAds() || mobileAdsInitialized ||
            mobileAdsInitializationInProgress
        ) {
            return
        }
        val configuration = RequestConfiguration.Builder()
            .setMaxAdContentRating(RequestConfiguration.MaxAdContentRating.MAX_AD_CONTENT_RATING_PG)
            .build()
        val initConfig = InitializationConfig.Builder("ca-app-pub-7040136510470731~9946661791")
            .setRequestConfiguration(configuration)
            .build()
        mobileAdsInitializationInProgress = true
        thread(name = "SyncPakAdsInit") {
            MobileAds.initialize(activity, initConfig) {
                activity.runOnUiThread {
                    if (destroyed) return@runOnUiThread
                    mobileAdsInitializationInProgress = false
                    mobileAdsInitialized = true
                    renderPlacement()
                }
            }
        }
    }

    private fun consentRequestParameters(): ConsentRequestParameters {
        val parameters = ConsentRequestParameters.Builder()
        if (!BuildConfig.UMP_FORCE_EEA_DEBUG_GEOGRAPHY) {
            return parameters.build()
        }

        val debugSettings = ConsentDebugSettings.Builder(activity)
            .setDebugGeography(ConsentDebugSettings.DebugGeography.DEBUG_GEOGRAPHY_EEA)
        BuildConfig.UMP_TEST_DEVICE_HASHED_IDS
            .split(',')
            .map(String::trim)
            .filter(String::isNotEmpty)
            .forEach(debugSettings::addTestDeviceHashedId)
        Log.i(LOG_TAG, "EEA consent debug geography enabled for this non-release build")
        return parameters.setConsentDebugSettings(debugSettings.build()).build()
    }

    private fun renderPlacement() {
        if (!canShow() || !mobileAdsInitialized || placement == Placement.HIDDEN) {
            removeAdvertising()
            return
        }
        if (advertisingContainer != null) {
            return
        }

        val adSize = anchoredBannerSize()
        val container = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
        }
        val adView = AdView(activity)
        container.addView(
            adView,
            LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                adSize.getHeightInPixels(activity),
            ),
        )
        container.addView(advertisingFooter())
        val popup = PopupWindow(
            container,
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT,
            false,
        ).apply {
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            isOutsideTouchable = false
            elevation = 8f
        }
        banner = adView
        advertisingContainer = container
        advertisingPopup = popup
        try {
            popup.showAtLocation(activity.window.decorView, Gravity.BOTTOM, 0, 0)
        } catch (error: WindowManager.BadTokenException) {
            // The window can be revoked between checking its token and adding the popup.
            Log.w(LOG_TAG, "Advertising window is no longer available", error)
            removeAdvertising()
            return
        }
        reportContainerInset(container)
        val adRequest = BannerAdRequest.Builder(BuildConfig.ADMOB_BANNER_AD_UNIT_ID, adSize).build()
        adView.loadAd(
            adRequest,
            object : AdLoadCallback<BannerAd> {
                override fun onAdLoaded(ad: BannerAd) {
                    activity.runOnUiThread {
                        if (destroyed || banner !== adView) return@runOnUiThread
                        logLoadedBannerLayout(container, adView)
                        reportContainerInset(container)
                    }
                }

                override fun onAdFailedToLoad(adError: LoadAdError) {
                    activity.runOnUiThread {
                        if (destroyed || banner !== adView) return@runOnUiThread
                        Log.w(
                            LOG_TAG,
                            "Banner ad failed to load: code=${adError.code}, " +
                                "message=${adError.message}, responseInfo=${adError.responseInfo}",
                        )
                        removeBannerButKeepFooter()
                    }
                }
            },
        )
    }

    private fun anchoredBannerSize(): AdSize {
        val metrics = activity.resources.displayMetrics
        val width = (metrics.widthPixels / metrics.density).toInt()
        return AdSize.getLargeAnchoredAdaptiveBannerAdSize(activity, width)
    }

    private fun logLoadedBannerLayout(container: LinearLayout, adView: AdView) {
        Log.i(
            LOG_TAG,
            "Banner loaded: container(parent=${container.parent != null}, attached=" +
                "${container.isAttachedToWindow}, shown=${container.isShown}, " +
                "size=${container.width}x${container.height}); banner(parent=${adView.parent != null}, " +
                "attached=${adView.isAttachedToWindow}, shown=${adView.isShown}, " +
                "size=${adView.width}x${adView.height})",
        )
    }

    private fun advertisingFooter(): TextView = TextView(activity).apply {
        text = "Banner ads will be removed from the app once you've set up at least one provider and connection."
        contentDescription = text
        gravity = Gravity.CENTER
        setPadding(24, 4, 24, 8)
        setTextColor(Color.DKGRAY)
        textSize = 12f
    }

    private fun removeBannerButKeepFooter() {
        val previousBanner = banner
        banner = null
        previousBanner?.let { view ->
            (view.parent as? ViewGroup)?.removeView(view)
            view.destroy()
        }
        advertisingContainer?.let(::reportContainerInset)
    }

    private fun removeAdvertising() {
        val previousBanner = banner
        val previousPopup = advertisingPopup
        banner = null
        advertisingContainer = null
        advertisingPopup = null
        previousBanner?.destroy()
        previousPopup?.dismiss()
        reportBannerInset(0)
    }

    private fun reportContainerInset(container: LinearLayout) {
        container.post {
            if (!destroyed && advertisingContainer === container && container.isAttachedToWindow) {
                reportBannerInset((container.height / activity.resources.displayMetrics.density).roundToInt())
            }
        }
    }

    private fun notifyPrivacyChoicesAvailability() {
        (activity as SyncPakActivity).advertisingPrivacyChoicesAvailabilityChanged(
            privacyChoicesAvailable,
        )
    }

    private fun reportBannerInset(heightDp: Int) {
        if (destroyed) return
        (activity as SyncPakActivity).advertisingBannerInsetChanged(heightDp)
    }

    private enum class Placement {
        HIDDEN,
        EMPTY_PROVIDERS,
        EMPTY_CONNECTIONS;

        companion object {
            fun from(value: Int): Placement = when (value) {
                1 -> EMPTY_PROVIDERS
                2 -> EMPTY_CONNECTIONS
                else -> HIDDEN
            }
        }
    }

    private companion object {
        const val LOG_TAG = "SyncPakAds"
    }
}
