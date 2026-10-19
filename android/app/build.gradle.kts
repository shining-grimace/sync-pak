plugins {
    id("com.android.application")
}

dependencies {
    // Keep this aligned with rustls-platform-verifier-android in Cargo.lock.
    implementation(libs.rustls.platform.verifier)
    implementation(libs.google.ads)
    implementation(libs.google.ump)
}

configurations.configureEach {
    exclude(group = "com.google.android.gms", module = "play-services-ads")
    exclude(group = "com.google.android.gms", module = "play-services-ads-lite")
}

android {
    namespace = "com.shininggrimace.syncpak"
    compileSdk {
        version = release(37) {
            minorApiLevel = 2
        }
    }

    defaultConfig {
        applicationId = "com.shininggrimace.syncpak"
        minSdk = 30
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    buildFeatures {
        buildConfig = true
    }

    buildTypes {
        getByName("debug") {
            buildConfigField(
                "String",
                "ADMOB_BANNER_AD_UNIT_ID",
                "\"ca-app-pub-3940256099942544/9214589741\"",
            )
            buildConfigField("boolean", "UMP_FORCE_EEA_DEBUG_GEOGRAPHY", "true")
            buildConfigField(
                "String",
                "UMP_TEST_DEVICE_HASHED_IDS",
                "\"F878ADE974DAC3613A248205085FF743\""
            )
        }
        getByName("release") {
            buildConfigField(
                "String",
                "ADMOB_BANNER_AD_UNIT_ID",
                "\"ca-app-pub-7040136510470731/1601267915\"",
            )
            buildConfigField("boolean", "UMP_FORCE_EEA_DEBUG_GEOGRAPHY", "false")
            buildConfigField(
                "String",
                "UMP_TEST_DEVICE_HASHED_IDS",
                "\"F878ADE974DAC3613A248205085FF743\""
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

apply(from = "rust-build.gradle.kts")
