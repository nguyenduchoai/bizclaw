/**
 * BizClaw LLM JNI Bridge
 *
 * Connects Kotlin BizClawLLM class to LLMInference C++ class.
 * JNI function names follow: Java_{package}_{class}_{method}
 *
 * Reference: SmolChat-Android/smollm/src/main/cpp/smollm.cpp
 */

#include "LLMInference.h"
#include <jni.h>

extern "C" JNIEXPORT jlong JNICALL
Java_vn_bizclaw_app_engine_BizClawLLM_loadModel(
        JNIEnv* env, jobject thiz, jstring modelPath, jfloat minP,
        jfloat temperature, jboolean storeChats, jlong contextSize,
        jstring chatTemplate, jint nThreads, jboolean useMmap, jboolean useMlock) {
    jboolean isCopy = true;
    const char* modelPathCstr = env->GetStringUTFChars(modelPath, &isCopy);
    const char* chatTemplateCstr = env->GetStringUTFChars(chatTemplate, &isCopy);
    auto* llmInference = new LLMInference();

    try {
        llmInference->loadModel(modelPathCstr, minP, temperature, storeChats,
                                contextSize, chatTemplateCstr, nThreads, useMmap, useMlock);
    } catch (std::exception& error) {
        env->ReleaseStringUTFChars(modelPath, modelPathCstr);
        env->ReleaseStringUTFChars(chatTemplate, chatTemplateCstr);
        delete llmInference;
        env->ThrowNew(env->FindClass("java/lang/IllegalStateException"), error.what());
        return 0;
    }

    env->ReleaseStringUTFChars(modelPath, modelPathCstr);
    env->ReleaseStringUTFChars(chatTemplate, chatTemplateCstr);
    return reinterpret_cast<jlong>(llmInference);
}

extern "C" JNIEXPORT void JNICALL
Java_vn_bizclaw_app_engine_BizClawLLM_addChatMessage(
        JNIEnv* env, jobject thiz, jlong modelPtr, jstring message, jstring role) {
    jboolean isCopy = true;
    const char* messageCstr = env->GetStringUTFChars(message, &isCopy);
    const char* roleCstr = env->GetStringUTFChars(role, &isCopy);
    auto* llmInference = reinterpret_cast<LLMInference*>(modelPtr);
    llmInference->addChatMessage(messageCstr, roleCstr);
    env->ReleaseStringUTFChars(message, messageCstr);
    env->ReleaseStringUTFChars(role, roleCstr);
}

extern "C" JNIEXPORT jfloat JNICALL
Java_vn_bizclaw_app_engine_BizClawLLM_getResponseGenerationSpeed(
        JNIEnv* env, jobject thiz, jlong modelPtr) {
    auto* llmInference = reinterpret_cast<LLMInference*>(modelPtr);
    return llmInference->getResponseGenerationTime();
}

extern "C" JNIEXPORT jint JNICALL
Java_vn_bizclaw_app_engine_BizClawLLM_getContextSizeUsed(
        JNIEnv* env, jobject thiz, jlong modelPtr) {
    auto* llmInference = reinterpret_cast<LLMInference*>(modelPtr);
    return llmInference->getContextSizeUsed();
}

extern "C" JNIEXPORT void JNICALL
Java_vn_bizclaw_app_engine_BizClawLLM_close(
        JNIEnv* env, jobject thiz, jlong modelPtr) {
    auto* llmInference = reinterpret_cast<LLMInference*>(modelPtr);
    delete llmInference;
}

extern "C" JNIEXPORT void JNICALL
Java_vn_bizclaw_app_engine_BizClawLLM_clearChatHistory(
        JNIEnv* env, jobject thiz, jlong modelPtr) {
    auto* llmInference = reinterpret_cast<LLMInference*>(modelPtr);
    llmInference->clearChatHistory();
}

extern "C" JNIEXPORT void JNICALL
Java_vn_bizclaw_app_engine_BizClawLLM_startCompletion(
        JNIEnv* env, jobject thiz, jlong modelPtr, jstring prompt) {
    jboolean isCopy = true;
    const char* promptCstr = env->GetStringUTFChars(prompt, &isCopy);
    auto* llmInference = reinterpret_cast<LLMInference*>(modelPtr);
    try {
        llmInference->startCompletion(promptCstr);
    } catch (std::exception& error) {
        env->ReleaseStringUTFChars(prompt, promptCstr);
        env->ThrowNew(env->FindClass("java/lang/IllegalStateException"), error.what());
        return;
    }
    env->ReleaseStringUTFChars(prompt, promptCstr);
}

extern "C" JNIEXPORT jstring JNICALL
Java_vn_bizclaw_app_engine_BizClawLLM_completionLoop(
        JNIEnv* env, jobject thiz, jlong modelPtr) {
    auto* llmInference = reinterpret_cast<LLMInference*>(modelPtr);
    try {
        std::string response = llmInference->completionLoop();
        return env->NewStringUTF(response.c_str());
    } catch (std::exception& error) {
        env->ThrowNew(env->FindClass("java/lang/IllegalStateException"), error.what());
        return nullptr;
    }
}

extern "C" JNIEXPORT void JNICALL
Java_vn_bizclaw_app_engine_BizClawLLM_stopCompletion(
        JNIEnv* env, jobject thiz, jlong modelPtr) {
    auto* llmInference = reinterpret_cast<LLMInference*>(modelPtr);
    llmInference->stopCompletion();
}

extern "C" JNIEXPORT jstring JNICALL
Java_vn_bizclaw_app_engine_BizClawLLM_benchModel(
        JNIEnv* env, jobject thiz, jlong modelPtr,
        jint pp, jint tg, jint pl, jint nr) {
    auto* llmInference = reinterpret_cast<LLMInference*>(modelPtr);
    std::string result = llmInference->benchModel(pp, tg, pl, nr);
    return env->NewStringUTF(result.c_str());
}
