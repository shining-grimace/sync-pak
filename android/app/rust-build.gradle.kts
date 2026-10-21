import com.android.build.api.dsl.ApplicationExtension
import com.android.build.api.variant.ApplicationAndroidComponentsExtension
import org.gradle.api.artifacts.VersionCatalogsExtension
import java.io.File
import java.util.Locale
import org.gradle.api.tasks.Sync
import org.gradle.api.tasks.Exec
import org.gradle.api.tasks.TaskProvider

val ndkVersion = "30.0.16248370"

buildscript {
    repositories {
        google()
        mavenCentral()
    }
    dependencies {
        val catalog = project.extensions.getByType<VersionCatalogsExtension>().named("libs")
        val androidGradlePlugin = catalog.findVersion("androidGradlePlugin").get()
        classpath("com.android.tools.build:gradle:$androidGradlePlugin")
    }
}

val android = extensions.getByType<ApplicationExtension>()
android.ndkVersion = ndkVersion
val androidComponents = extensions.getByType<ApplicationAndroidComponentsExtension>()
val repositoryRoot = rootProject.projectDir.parentFile
val androidSdkDirectory = androidComponents.sdkComponents.sdkDirectory
val rustTargets = mapOf(
    "arm64-v8a" to "aarch64-linux-android",
    "x86_64" to "x86_64-linux-android",
)

fun Exec.configureRustEnvironment(rustTarget: String) {
    val targetEnvironment = rustTarget.replace('-', '_')
    val sdkRoot = androidSdkDirectory.get().asFile.absolutePath
    val ndkDirectory = File(sdkRoot, "ndk/$ndkVersion")
    if (!ndkDirectory.isDirectory) {
        throw GradleException(
            "Android NDK $ndkVersion is not installed. " +
                "Install it with sdkmanager \"ndk;$ndkVersion\".",
        )
    }
    val ndkRoot = ndkDirectory.absolutePath
    val osName = System.getProperty("os.name").lowercase(Locale.ROOT)
    val host = when {
        osName.contains("mac") -> "darwin-x86_64"
        osName.contains("win") -> "windows-x86_64"
        else -> "linux-x86_64"
    }
    val commandSuffix = if (osName.contains("win")) ".cmd" else ""
    val binarySuffix = if (osName.contains("win")) ".exe" else ""
    val toolchain = "$ndkRoot/toolchains/llvm/prebuilt/$host/bin"
    val linker = "$toolchain/$rustTarget${android.defaultConfig.minSdk}-clang$commandSuffix"
    val cxx = "$toolchain/$rustTarget${android.defaultConfig.minSdk}-clang++$commandSuffix"
    val ar = "$toolchain/llvm-ar$binarySuffix"
    val javaHome = System.getProperty("java.home")
    val path = listOf(
        toolchain,
        File(javaHome, "bin").absolutePath,
        System.getenv("PATH") ?: "",
    ).filter { it.isNotEmpty() }.joinToString(File.pathSeparator)
    environment("JAVA_HOME", javaHome)
    environment("ANDROID_HOME", sdkRoot)
    environment("ANDROID_SDK_ROOT", sdkRoot)
    environment("ANDROID_NDK_HOME", ndkRoot)
    environment("ANDROID_NDK_ROOT", ndkRoot)
    environment("ANDROID_PLATFORM", "android-37.0")
    // Align the Rust shared library for devices using 16 KB pages.
    val rustFlags = System.getenv("CARGO_ENCODED_RUSTFLAGS")
        ?.split('\u001f')?.filter { it.isNotEmpty() }
        ?: System.getenv("RUSTFLAGS")?.split(Regex("\\s+"))?.filter { it.isNotEmpty() }
        ?: emptyList()
    environment(
        "CARGO_ENCODED_RUSTFLAGS",
        (rustFlags + listOf("-C", "link-arg=-Wl,-z,max-page-size=16384"))
            .joinToString("\u001f"),
    )
    environment("CARGO_TARGET_${targetEnvironment.uppercase(Locale.ROOT)}_LINKER", linker)
    environment("CC_$targetEnvironment", linker)
    environment("CXX_$targetEnvironment", cxx)
    environment("AR_$targetEnvironment", ar)
    environment("PATH", path)
}

fun registerRustTasks(
    buildType: String,
    buildSuffix: String,
    abi: String,
    rustTarget: String,
): Pair<TaskProvider<Exec>, TaskProvider<Sync>> {
    val release = buildType == "release"
    val taskSuffix = if (abi == "arm64-v8a") "Arm64" else "X86_64"
    val rustLibrary = File(repositoryRoot, "target/$rustTarget/$buildType/libsync_pak.so")
    val generatedLibraries = layout.buildDirectory.dir("generated/rust-libs/$buildType/$abi")

    val buildRust = tasks.register<Exec>("buildRust$buildSuffix$taskSuffix") {
        workingDir(repositoryRoot)
        val cargoArguments = mutableListOf(
            "cargo", "build", "--locked", "--lib", "--target", rustTarget,
        )
        if (release) {
            dependsOn("validateSigningRelease")
            cargoArguments += "--release"
        } else if (providers.gradleProperty("feasibilityProbes").orNull == "true") {
            cargoArguments += listOf("--features", "feasibility-probes")
        }
        commandLine(cargoArguments)

        doFirst {
            configureRustEnvironment(rustTarget)
        }
    }

    val stageRust = tasks.register<Sync>("stageRust$buildSuffix$taskSuffix") {
        dependsOn(buildRust)
        from(rustLibrary)
        into(generatedLibraries)
    }
    return buildRust to stageRust
}

mapOf("debug" to "Debug", "release" to "Release").forEach { (buildType, buildSuffix) ->
    val generatedRoot = layout.buildDirectory.dir("generated/rust-libs/$buildType")
    android.sourceSets.getByName(buildType).jniLibs.directories
        .add(
            generatedRoot.get().asFile.absolutePath
        )
    val buildAll = tasks.register("buildRust$buildSuffix")
    val stageAll = tasks.register("stageRust$buildSuffix")

    rustTargets.forEach { (abi, rustTarget) ->
        val (buildRust, stageRust) = registerRustTasks(buildType, buildSuffix, abi, rustTarget)
        buildAll.configure {
            dependsOn(buildRust)
        }
        stageAll.configure {
            dependsOn(stageRust)
        }
    }

    tasks.configureEach {
        if (name == "merge${buildSuffix}JniLibFolders" || name == "merge${buildSuffix}NativeLibs") {
            dependsOn(stageAll)
        }
    }
}
