plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val bizClawAndroidBuildRoot = System.getenv("BIZCLAW_ANDROID_BUILD_ROOT")
    ?: "${System.getProperty("user.home")}/.cache/bizclaw/android-build"
layout.buildDirectory.set(file("$bizClawAndroidBuildRoot/companion"))

android {
    namespace = "vn.bizclaw.companion"
    compileSdk = 35

    defaultConfig {
        applicationId = "vn.bizclaw.companion"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.1.7"

        buildConfigField("String", "DEFAULT_SERVER_URL", "\"http://10.0.2.2:3001\"")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        buildConfig = true
    }
}

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
}

tasks.configureEach {
    val clearsAppleDouble =
        name.contains("resource", ignoreCase = true) ||
        name.contains("manifest", ignoreCase = true) ||
        name.contains("kotlin", ignoreCase = true) ||
        name.contains("javac", ignoreCase = true) ||
        name.contains("dex", ignoreCase = true)
    if (
        clearsAppleDouble
    ) {
        doFirst {
            delete(fileTree(layout.buildDirectory) {
                include("**/._*")
            })
            delete(fileTree("src") {
                include("**/._*")
            })
        }
        doLast {
            delete(fileTree(layout.buildDirectory) {
                include("**/._*")
            })
            delete(fileTree("src") {
                include("**/._*")
            })
        }
    }
}
