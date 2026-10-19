import com.android.build.api.dsl.ApplicationExtension
import com.android.build.api.variant.ApplicationAndroidComponentsExtension
import org.gradle.api.artifacts.VersionCatalogsExtension
import java.io.File
import java.util.Locale
import org.gradle.api.tasks.Copy
import org.gradle.api.tasks.Exec

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
val androidComponents = extensions.getByType<ApplicationAndroidComponentsExtension>()
val generatedLibrariesRoot = layout.buildDirectory.dir("generated/rust-libs").get().asFile.absolutePath
android.sourceSets.getByName("main").jniLibs.directories.add(generatedLibrariesRoot)

val repositoryRoot = rootProject.projectDir.parentFile
val androidSdkDirectory = androidComponents.sdkComponents.sdkDirectory

val buildRustDebug = tasks.register("buildRustDebug")
val stageRustDebug = tasks.register("stageRustDebug")

val rustTargets = mapOf(
    "arm64-v8a" to "aarch64-linux-android",
    "x86_64" to "x86_64-linux-android",
)

rustTargets.forEach { (abi, rustTarget) ->
    val taskSuffix = if (abi == "arm64-v8a") "Arm64" else "X86_64"
    val targetEnvironment = rustTarget.replace('-', '_')
    val rustLibrary = File(repositoryRoot, "target/$rustTarget/debug/libsync_pak.so")
    val generatedLibraries = layout.buildDirectory.dir("generated/rust-libs/$abi")

    val buildRust = tasks.register<Exec>("buildRustDebug$taskSuffix") {
        workingDir(repositoryRoot)
        val cargoArguments = mutableListOf(
            "cargo", "build", "--locked", "--lib", "--target", rustTarget,
        )
        if (providers.gradleProperty("feasibilityProbes").orNull == "true") {
            cargoArguments += listOf("--features", "feasibility-probes")
        }
        commandLine(cargoArguments)

        doFirst {
            val sdkRoot = androidSdkDirectory.get().asFile.absolutePath
            val ndkRoot = System.getenv("ANDROID_NDK_HOME")
                ?: System.getenv("ANDROID_NDK_ROOT")
                ?: File(sdkRoot, "ndk").listFiles()
                    ?.filter { it.isDirectory }
                    ?.maxByOrNull { it.name }
                    ?.absolutePath
                ?: throw GradleException("No side-by-side Android NDK installation was found")
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
            environment("ANDROID_PLATFORM", "android-37.0")
            environment("CARGO_TARGET_${targetEnvironment.uppercase(Locale.ROOT)}_LINKER", linker)
            environment("CC_$targetEnvironment", linker)
            environment("CXX_$targetEnvironment", cxx)
            environment("AR_$targetEnvironment", ar)
            environment("PATH", path)
        }
    }

    val stageRust = tasks.register<Copy>("stageRustDebug$taskSuffix") {
        dependsOn(buildRust)
        from(rustLibrary)
        into(generatedLibraries)
    }

    buildRustDebug.configure { dependsOn(buildRust) }
    stageRustDebug.configure { dependsOn(stageRust) }
}

tasks.configureEach {
    if (name == "mergeDebugJniLibFolders" || name == "mergeDebugNativeLibs") {
        dependsOn(stageRustDebug)
    }
}
