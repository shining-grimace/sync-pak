import org.gradle.api.tasks.Copy
import org.gradle.api.tasks.Exec

plugins {
    id("com.android.application")
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

    sourceSets.getByName("main").jniLibs.directories.add(
        layout.buildDirectory.dir("generated/rust-libs").get().asFile.absolutePath,
    )
}

val repositoryRoot = rootProject.projectDir.parentFile
val rustTarget = "aarch64-linux-android"
val rustLibrary = repositoryRoot.resolve("target/$rustTarget/debug/libsync_pak.so")
val generatedLibraries = layout.buildDirectory.dir("generated/rust-libs/arm64-v8a")

val buildRustDebug by tasks.registering(Exec::class) {
    workingDir(repositoryRoot)
    val cargoArguments = mutableListOf(
        "cargo", "build", "--locked", "--lib", "--target", rustTarget,
    )
    if (providers.gradleProperty("feasibilityProbes").orNull == "true") {
        cargoArguments += listOf("--features", "feasibility-probes")
    }
    commandLine(cargoArguments)

    doFirst {
        val sdkRoot = System.getenv("ANDROID_HOME")
            ?: System.getenv("ANDROID_SDK_ROOT")
            ?: File(System.getProperty("user.home"), "Android/Sdk").absolutePath
        val ndkRoot = System.getenv("ANDROID_NDK_HOME")
            ?: System.getenv("ANDROID_NDK_ROOT")
            ?: File(sdkRoot, "ndk").listFiles()
                ?.filter(File::isDirectory)
                ?.maxByOrNull(File::getName)
                ?.absolutePath
            ?: throw GradleException("No side-by-side Android NDK installation was found")
        val osName = System.getProperty("os.name").lowercase()
        val host = when {
            osName.contains("mac") -> "darwin-x86_64"
            osName.contains("win") -> "windows-x86_64"
            else -> "linux-x86_64"
        }
        val commandSuffix = if (osName.contains("win")) ".cmd" else ""
        val binarySuffix = if (osName.contains("win")) ".exe" else ""
        val toolchain = "$ndkRoot/toolchains/llvm/prebuilt/$host/bin"
        val linker = "$toolchain/aarch64-linux-android30-clang$commandSuffix"
        val cxx = "$toolchain/aarch64-linux-android30-clang++$commandSuffix"
        val ar = "$toolchain/llvm-ar$binarySuffix"
        environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", linker)
        environment("CC_aarch64_linux_android", linker)
        environment("CXX_aarch64_linux_android", cxx)
        environment("AR_aarch64_linux_android", ar)
        environment("PATH", "$toolchain${File.pathSeparator}${System.getenv("PATH")}")
    }
}

val stageRustDebug by tasks.registering(Copy::class) {
    dependsOn(buildRustDebug)
    from(rustLibrary)
    into(generatedLibraries)
}

tasks.configureEach {
    if (name == "mergeDebugJniLibFolders" || name == "mergeDebugNativeLibs") {
        dependsOn(stageRustDebug)
    }
}
