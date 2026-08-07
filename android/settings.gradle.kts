import kotlinx.serialization.json.Json

pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

buildscript {
    repositories {
        mavenCentral()
    }
    dependencies {
        classpath("org.jetbrains.kotlinx:kotlinx-serialization-json:1.11.0")
    }
}

plugins {
    id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
        rustlsPlatformVerifier()
    }
}

fun RepositoryHandler.rustlsPlatformVerifier(): MavenArtifactRepository = maven {
    // Gradle requires a URL to be specified. Add the actual URL here.
    url = uri("https://example.com")
    // Placeholder usage
    val json = Json { ignoreUnknownKeys = true }
}

rootProject.name = "SyncPak"
include(":app")
