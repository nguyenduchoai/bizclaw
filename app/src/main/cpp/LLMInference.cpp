/**
 * LLMInference — llama.cpp wrapper for BizClaw Android
 * 
 * Based on SmolChat-Android's LLMInference.cpp
 * Reference: github.com/shubham0204/SmolChat-Android
 *
 * This handles:
 * 1. Load GGUF model with mmap/mlock
 * 2. Chat template formatting (Jinja2)
 * 3. Streaming token generation
 * 4. Benchmarking
 */

#include "LLMInference.h"
#include <android/log.h>
#include <cmath>
#include <iomanip>
#include <sstream>
#include <stdexcept>

static const char* TAG = "BizClawLLM";
#define LOGi(...) __android_log_print(ANDROID_LOG_INFO, TAG, __VA_ARGS__)
#define LOGe(...) __android_log_print(ANDROID_LOG_ERROR, TAG, __VA_ARGS__)

void
LLMInference::loadModel(const char *model_path, float minP, float temperature, bool storeChats,
                         long contextSize, const char *chatTemplate, int nThreads,
                         bool useMmap, bool useMlock) {
    LOGi("loading model with"
         "\n\tmodel_path = %s"
         "\n\tminP = %f"
         "\n\ttemperature = %f"
         "\n\tstoreChats = %d"
         "\n\tcontextSize = %li"
         "\n\tnThreads = %d"
         "\n\tuseMmap = %d"
         "\n\tuseMlock = %d",
         model_path, minP, temperature, storeChats, contextSize, nThreads, useMmap, useMlock);

    // Load dynamic backends
    ggml_backend_load_all();

    // Load model
    llama_model_params model_params = llama_model_default_params();
    model_params.use_mmap = useMmap;
    model_params.use_mlock = useMlock;
    _model = llama_model_load_from_file(model_path, model_params);
    if (!_model) {
        LOGe("failed to load model from %s", model_path);
        throw std::runtime_error("loadModel() failed");
    }

    // Create context
    llama_context_params ctx_params = llama_context_default_params();
    ctx_params.n_ctx = contextSize;
    ctx_params.n_batch = contextSize;
    ctx_params.n_threads = nThreads;
    ctx_params.no_perf = true;
    _ctx = llama_init_from_model(_model, ctx_params);
    if (!_ctx) {
        LOGe("llama_init_from_model() returned null");
        throw std::runtime_error("llama_init_from_model() returned null");
    }

    // Create sampler chain
    llama_sampler_chain_params sampler_params = llama_sampler_chain_default_params();
    sampler_params.no_perf = true;
    _sampler = llama_sampler_chain_init(sampler_params);
    llama_sampler_chain_add(_sampler, llama_sampler_init_temp(temperature));
    llama_sampler_chain_add(_sampler, llama_sampler_init_dist(LLAMA_DEFAULT_SEED));

    _formattedMessages = std::vector<char>(llama_n_ctx(_ctx));
    _messages.clear();

    if (chatTemplate == nullptr) {
        _chatTemplate = llama_model_chat_template(_model, nullptr);
    } else {
        _chatTemplate = strdup(chatTemplate);
    }
    this->_storeChats = storeChats;
    
    LOGi("✅ Model loaded successfully (ctx=%ld)", contextSize);
}

void
LLMInference::addChatMessage(const char *message, const char *role) {
    _messages.push_back({strdup(role), strdup(message)});
}

void
LLMInference::clearChatHistory() {
    _messages.clear();
    _formattedMessages.clear();
    _formattedMessages = std::vector<char>(llama_n_ctx(_ctx));
    _nCtxUsed = 0;
    LOGi("Chat history cleared");
}

float
LLMInference::getResponseGenerationTime() const {
    return (float) _responseNumTokens / (_responseGenerationTime / 1e6);
}

int
LLMInference::getContextSizeUsed() const {
    return _nCtxUsed;
}

void
LLMInference::startCompletion(const char *query) {
    if (!_storeChats) {
        _formattedMessages.clear();
        _formattedMessages = std::vector<char>(llama_n_ctx(_ctx));
    }
    _responseGenerationTime = 0;
    _responseNumTokens = 0;
    addChatMessage(query, "user");

    // Apply chat template
    std::vector<common_chat_msg> messages;
    for (const llama_chat_message& message : _messages) {
        common_chat_msg msg;
        msg.role    = message.role;
        msg.content = message.content;
        messages.push_back(msg);
    }
    common_chat_templates_inputs inputs;
    inputs.use_jinja = true;
    inputs.messages  = messages;
    auto        templates = common_chat_templates_init(_model, _chatTemplate);
    std::string prompt    = common_chat_templates_apply(templates.get(), inputs).prompt;
    _promptTokens = common_tokenize(llama_model_get_vocab(_model), prompt, true, true);

    // Create batch for prompt tokens
    _batch = new llama_batch();
    _batch->token = _promptTokens.data();
    _batch->n_tokens = _promptTokens.size();
}

bool
LLMInference::_isValidUtf8(const char *response) {
    if (!response) return true;
    const unsigned char *bytes = (const unsigned char *) response;
    int num;
    while (*bytes != 0x00) {
        if ((*bytes & 0x80) == 0x00) { num = 1; }
        else if ((*bytes & 0xE0) == 0xC0) { num = 2; }
        else if ((*bytes & 0xF0) == 0xE0) { num = 3; }
        else if ((*bytes & 0xF8) == 0xF0) { num = 4; }
        else { return false; }
        bytes += 1;
        for (int i = 1; i < num; ++i) {
            if ((*bytes & 0xC0) != 0x80) return false;
            bytes += 1;
        }
    }
    return true;
}

std::string
LLMInference::completionLoop() {
    uint32_t contextSize = llama_n_ctx(_ctx);
    _nCtxUsed = llama_memory_seq_pos_max(llama_get_memory(_ctx), 0) + 1;
    if (_nCtxUsed + _batch->n_tokens > contextSize) {
        throw std::runtime_error("context size reached");
    }

    auto start = ggml_time_us();
    if (llama_decode(_ctx, *_batch) < 0) {
        throw std::runtime_error("llama_decode() failed");
    }

    _currToken = llama_sampler_sample(_sampler, _ctx, -1);
    if (llama_vocab_is_eog(llama_model_get_vocab(_model), _currToken)) {
        addChatMessage(strdup(_response.data()), "assistant");
        _response.clear();
        return "[EOG]";
    }
    std::string piece = common_token_to_piece(_ctx, _currToken, true);
    auto end = ggml_time_us();
    _responseGenerationTime += (end - start);
    _responseNumTokens += 1;
    _cacheResponseTokens += piece;

    // Re-init batch with predicted token (KV cache reuse)
    _batch->token = &_currToken;
    _batch->n_tokens = 1;

    if (_isValidUtf8(_cacheResponseTokens.c_str())) {
        _response += _cacheResponseTokens;
        std::string valid_utf8_piece = _cacheResponseTokens;
        _cacheResponseTokens.clear();
        return valid_utf8_piece;
    }

    return "";
}

void
LLMInference::stopCompletion() {
    if (_storeChats) {
        addChatMessage(_response.c_str(), "assistant");
    }
    _response.clear();
}

LLMInference::~LLMInference() {
    for (llama_chat_message& message : _messages) {
        free(const_cast<char*>(message.role));
        free(const_cast<char*>(message.content));
    }
    llama_free(_ctx);
    llama_model_free(_model);
    delete _batch;
    llama_sampler_free(_sampler);
}

std::string
LLMInference::benchModel(int pp, int tg, int pl, int nr) {
    g_batch = llama_batch_init(pp, 0, pl);
    auto pp_avg = 0.0, tg_avg = 0.0, pp_std = 0.0, tg_std = 0.0;
    const uint32_t n_ctx = llama_n_ctx(this->_ctx);

    for (int nri = 0; nri < nr; nri++) {
        common_batch_clear(g_batch);
        for (int i = 0; i < pp; i++) {
            common_batch_add(g_batch, 1, i, {0}, false);
        }
        g_batch.logits[g_batch.n_tokens - 1] = true;
        llama_memory_clear(llama_get_memory(this->_ctx), false);

        const auto t_pp_start = ggml_time_us();
        llama_decode(this->_ctx, g_batch);
        const auto t_pp_end = ggml_time_us();

        llama_memory_clear(llama_get_memory(this->_ctx), false);
        const auto t_tg_start = ggml_time_us();
        for (int i = 0; i < tg; i++) {
            common_batch_clear(g_batch);
            for (int j = 0; j < pl; j++) {
                common_batch_add(g_batch, 0, i, {j}, true);
            }
            llama_decode(this->_ctx, g_batch);
        }
        const auto t_tg_end = ggml_time_us();
        llama_memory_clear(llama_get_memory(this->_ctx), false);

        const auto t_pp = double(t_pp_end - t_pp_start) / 1000000.0;
        const auto t_tg = double(t_tg_end - t_tg_start) / 1000000.0;
        const auto speed_pp = double(pp) / t_pp;
        const auto speed_tg = double(pl * tg) / t_tg;

        pp_avg += speed_pp; tg_avg += speed_tg;
        pp_std += speed_pp * speed_pp; tg_std += speed_tg * speed_tg;
    }

    llama_batch_free(g_batch);
    pp_avg /= double(nr); tg_avg /= double(nr);
    if (nr > 1) {
        pp_std = sqrt(pp_std / double(nr-1) - pp_avg*pp_avg*double(nr)/double(nr-1));
        tg_std = sqrt(tg_std / double(nr-1) - tg_avg*tg_avg*double(nr)/double(nr-1));
    } else {
        pp_std = 0; tg_std = 0;
    }

    char model_desc[128];
    llama_model_desc(this->_model, model_desc, sizeof(model_desc));
    const auto model_size = double(llama_model_size(this->_model)) / 1024.0 / 1024.0 / 1024.0;
    const auto model_n_params = double(llama_model_n_params(this->_model)) / 1e9;

    std::stringstream result;
    result << std::setprecision(3);
    result << "| model | size | params | test | t/s |\n";
    result << "| --- | --- | --- | --- | --- |\n";
    result << "| " << model_desc << " | " << model_size << "GiB | " << model_n_params << "B | pp "
           << pp << " | " << pp_avg << " ± " << pp_std << " |\n";
    result << "| " << model_desc << " | " << model_size << "GiB | " << model_n_params << "B | tg "
           << tg << " | " << tg_avg << " ± " << tg_std << " |\n";
    return result.str();
}
