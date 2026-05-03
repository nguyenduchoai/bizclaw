plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jetbrains.kotlin.plugin.serialization")
}

android {
    namespace = "vn.bizclaw.app"
    compileSdk = 35

    defaultConfig {
        applicationId = "vn.bizclaw.app"
        minSdk = 26
        targetSdk = 35
        versionCode = 19
        versionName = "1.1.7"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        // BizClaw server defaults
        buildConfigField("String", "DEFAULT_SERVER_URL", "\"http://localhost:3001\"")
        buildConfigField("String", "APP_VERSION", "\"1.1.7\"")

        // NDK — only ARM64 (primary) + x86_64 (emulator)
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    // llama.cpp engine build via CMake (on-device LLM inference)
    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }

    defaultConfig {
        externalNativeBuild {
            cmake {
                arguments += listOf("-DANDROID_SUPPORT_FLEXIBLE_PAGE_SIZES=ON")
                arguments += "-DCMAKE_BUILD_TYPE=Release"
                arguments += "-DBUILD_SHARED_LIBS=ON"
                arguments += "-DLLAMA_BUILD_COMMON=ON"
                arguments += "-DLLAMA_CURL=OFF"
                arguments += "-DGGML_LLAMAFILE=OFF"
            }
        }
    }

    signingConfigs {
        create("release") {
            storeFile = file("../release-keystore.jks")
            storePassword = System.getenv("BIZCLAW_STORE_PASSWORD") ?: "bizclaw2026"
            keyAlias = "bizclaw"
            keyPassword = System.getenv("BIZCLAW_KEY_PASSWORD") ?: "bizclaw2026"
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            signingConfig = signingConfigs.getByName("release")
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }
}

dependencies {
    // Compose BOM
    val composeBom = platform("androidx.compose:compose-bom:2024.12.01")
    implementation(composeBom)
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material:material-icons-extended")

    // Activity + Navigation
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.navigation:navigation-compose:2.8.5")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")

    // Networking — OkHttp + Retrofit
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.squareup.okhttp3:okhttp-sse:4.12.0")
    implementation("com.squareup.retrofit2:retrofit:2.11.0")
    implementation("com.squareup.retrofit2:converter-kotlinx-serialization:2.11.0")

    // JSON
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.7.3")

    // Security — Encrypted SharedPreferences
    implementation("androidx.security:security-crypto:1.1.0-alpha06")

    // Biometric Authentication
    implementation("androidx.biometric:biometric:1.1.0")

    // CameraX for Vision Activity
    implementation("androidx.camera:camera-core:1.3.4")
    implementation("androidx.camera:camera-camera2:1.3.4")
    implementation("androidx.camera:camera-lifecycle:1.3.4")
    implementation("androidx.camera:camera-view:1.3.4")

    // CardView
    implementation("androidx.cardview:cardview:1.0.0")

    // RecyclerView
    implementation("androidx.recyclerview:recyclerview:1.3.2")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")

    // LiteRT-LM (Google AI Edge) for Gemma 4 & Agent Calling natively without JNI crashes
    implementation("com.google.ai.edge.litertlm:litertlm-android:0.10.0")

    // Testing
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    debugImplementation("androidx.compose.ui:ui-tooling")
}
