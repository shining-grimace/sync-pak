import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

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

fun RepositoryHandler.rustlsPlatformVerifier(): MavenArtifactRepository {
    @Suppress("UnstableApiUsage")
    val manifestPath = let {
        val dependencyJson = providers.exec {
            workingDir = rootDir.parentFile
            commandLine("cargo", "metadata", "--format-version", "1", "--filter-platform", "aarch64-linux-android", "--manifest-path", "Cargo.toml")
        }.standardOutput.asText

        val path = Json.decodeFromString<JsonObject>(dependencyJson.get())
            .getValue("packages")
            .jsonArray
            .first { element ->
                element.jsonObject.getValue("name").jsonPrimitive.content == "rustls-platform-verifier-android"
            }.jsonObject.getValue("manifest_path").jsonPrimitive.content

        File(path)
    }

    return maven {
        url = uri(File(manifestPath.parentFile, "maven").path)
        metadataSources.artifact()
    }
}

rootProject.name = "SyncPak"
include(":app")
