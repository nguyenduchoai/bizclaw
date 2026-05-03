#pragma once
#include "chat.h"
#include "common.h"
#include "llama.h"
#include <string>
#include <vector>

/**
 * LLMInference — wraps llama.cpp for on-device GGUF model inference
 *
 * This is a C++ class that manages:
 * - Model loading with mmap/mlock
 * - Chat history with template formatting
 * - Streaming token generation
 * - Performance metrics tracking
 *
 * Thread safety: single-threaded use per instance.
 * Thread pool for matmul is handled by llama.cpp internally.
 */
class LLMInference {
    // llama.cpp handles
    llama_context* _ctx = nullptr;
    llama_model*   _model = nullptr;
    llama_sampler* _sampler = nullptr;
    llama_token    _currToken;
    llama_batch*   _batch = nullptr;
    llama_batch    g_batch;

    // Chat messages for template formatting
    std::vector<llama_chat_message> _messages;
    std::vector<char>              _formattedMessages;
    std::vector<llama_token>       _promptTokens;
    const char*                    _chatTemplate;

    // Response state
    std::string _response;
    std::string _cacheResponseTokens;
    bool        _storeChats;

    // Performance metrics
    int64_t _responseGenerationTime = 0;
    long    _responseNumTokens = 0;
    int     _nCtxUsed = 0;

    bool _isValidUtf8(const char* response);

public:
    void loadModel(const char* modelPath, float minP, float temperature,
                   bool storeChats, long contextSize, const char* chatTemplate,
                   int nThreads, bool useMmap, bool useMlock);

    std::string benchModel(int pp, int tg, int pl, int nr);

    void addChatMessage(const char* message, const char* role);
    void clearChatHistory();
    float getResponseGenerationTime() const;
    int getContextSizeUsed() const;

    void startCompletion(const char* query);
    std::string completionLoop();
    void stopCompletion();

    ~LLMInference();
};
