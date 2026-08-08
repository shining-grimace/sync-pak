plugins {
    id("com.android.application")
}

dependencies {
    // Keep this aligned with rustls-platform-verifier-android in Cargo.lock.
    implementation(libs.rustls.platform.verifier)
}

android {
    namespace = "com.shininggrimace.syncpak"
    compileSdk {
        version = release(36) {
            minorApiLevel = 1
        }
    }

    defaultConfig {
        applicationId = "com.shininggrimace.syncpak"
        minSdk = 30
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"

        ndk {
            abiFilters += "arm64-v8a"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

apply(from = "rust-build.gradle")
