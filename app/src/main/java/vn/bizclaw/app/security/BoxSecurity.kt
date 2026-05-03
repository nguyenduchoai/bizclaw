package vn.bizclaw.app.security

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

class BoxSecurity(private val context: Context) {
    
    private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    
    private var biometricEnabled = false
    private var encryptionEnabled = true
    private var hardOfflineMode = false
    
    private val keyAlias = "box_security_key"
    
    fun isBiometricAvailable(): BiometricStatus {
        val biometricManager = BiometricManager.from(context)
        
        return when (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)) {
            BiometricManager.BIOMETRIC_SUCCESS -> BiometricStatus.AVAILABLE
            BiometricManager.BIOMETRIC_ERROR_NO_HARDWARE -> BiometricStatus.NO_HARDWARE
            BiometricManager.BIOMETRIC_ERROR_HW_UNAVAILABLE -> BiometricStatus.UNAVAILABLE
            BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED -> BiometricStatus.NOT_ENROLLED
            else -> BiometricStatus.UNAVAILABLE
        }
    }
    
    suspend fun enableBiometric(activity: FragmentActivity): Result<Unit> = withContext(Dispatchers.Main) {
        try {
            val status = isBiometricAvailable()
            if (status != BiometricStatus.AVAILABLE) {
                return@withContext Result.failure(Exception("Biometric not available: $status"))
            }
            biometricEnabled = true
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    suspend fun authenticate(
        activity: FragmentActivity,
        reason: String = "Unlock Box AI"
    ): Result<BiometricResult> = withContext(Dispatchers.Main) {
        try {
            val executor = ContextCompat.getMainExecutor(context)
            
            val callback = object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    super.onAuthenticationSucceeded(result)
                }
                
                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                    super.onAuthenticationError(errorCode, errString)
                }
                
                override fun onAuthenticationFailed() {
                    super.onAuthenticationFailed()
                }
            }
            
            val biometricPrompt = BiometricPrompt(activity, executor, callback)
            
            val promptInfo = BiometricPrompt.PromptInfo.Builder()
                .setTitle("Box AI Security")
                .setSubtitle(reason)
                .setNegativeButtonText("Cancel")
                .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG)
                .build()
            
            biometricPrompt.authenticate(promptInfo)
            Result.success(BiometricResult.Success)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    suspend fun encrypt(data: ByteArray): Result<EncryptedData> = withContext(Dispatchers.IO) {
        try {
            val key = getOrCreateKey()
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.ENCRYPT_MODE, key)
            
            val encrypted = cipher.doFinal(data)
            val iv = cipher.iv
            
            Result.success(EncryptedData(encrypted, iv))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    suspend fun decrypt(encrypted: EncryptedData): Result<ByteArray> = withContext(Dispatchers.IO) {
        try {
            val key = getOrCreateKey()
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            val spec = GCMParameterSpec(128, encrypted.iv)
            cipher.init(Cipher.DECRYPT_MODE, key, spec)
            
            val decrypted = cipher.doFinal(encrypted.data)
            Result.success(decrypted)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    suspend fun encryptString(text: String): Result<String> {
        return encrypt(text.toByteArray()).map { enc ->
            android.util.Base64.encodeToString(enc.data, android.util.Base64.NO_WRAP) + ":" + 
            android.util.Base64.encodeToString(enc.iv, android.util.Base64.NO_WRAP)
        }
    }
    
    suspend fun decryptString(encrypted: String): Result<String> {
        return try {
            val parts = encrypted.split(":")
            if (parts.size != 2) {
                return Result.failure(Exception("Invalid encrypted format"))
            }
            
            val data = android.util.Base64.decode(parts[0], android.util.Base64.NO_WRAP)
            val iv = android.util.Base64.decode(parts[1], android.util.Base64.NO_WRAP)
            
            decrypt(EncryptedData(data, iv)).map { String(it) }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    fun enableHardOfflineMode(enable: Boolean) {
        hardOfflineMode = enable
    }
    
    fun isHardOfflineMode() = hardOfflineMode
    
    private fun getOrCreateKey(): SecretKey {
        val existingKey = keyStore.getEntry(keyAlias, null) as? KeyStore.SecretKeyEntry
        if (existingKey != null) {
            return existingKey.secretKey
        }
        
        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            "AndroidKeyStore"
        )
        
        val keySpec = KeyGenParameterSpec.Builder(
            keyAlias,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setUserAuthenticationRequired(false)
            .build()
        
        keyGenerator.init(keySpec)
        return keyGenerator.generateKey()
    }
    
    suspend fun encryptChatHistory(messages: List<ChatMessage>): Result<String> {
        return try {
            val json = serializeChatMessages(messages)
            encryptString(json)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    suspend fun decryptChatHistory(encrypted: String): Result<List<ChatMessage>> {
        return try {
            decryptString(encrypted).mapCatching { json ->
                deserializeChatMessages(json)
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
    
    private fun serializeChatMessages(messages: List<ChatMessage>): String {
        val items = messages.joinToString(",") { msg ->
            """{"id":"${msg.id.escapeJson()}","role":"${msg.role.escapeJson()}","content":"${msg.content.escapeJson()}","timestamp":${msg.timestamp}}"""
        }
        return "[$items]"
    }
    
    private fun deserializeChatMessages(json: String): List<ChatMessage> {
        if (json.isBlank() || json == "[]") return emptyList()
        
        val result = mutableListOf<ChatMessage>()
        val regex = """\{"id":"([^"]*)","role":"([^"]*)","content":"([^"]*)","timestamp":(\d+)\}""".toRegex()
        
        var remaining = json.trim().removePrefix("[").removeSuffix("]")
        while (remaining.isNotBlank()) {
            val match = regex.find(remaining)
            if (match != null) {
                val (id, role, content, timestamp) = match.destructured
                result.add(ChatMessage(id, role, content, timestamp.toLong()))
                remaining = remaining.substringAfter(match.value).trimStart(',', ' ')
            } else {
                break
            }
        }
        return result
    }
    
    private fun String.escapeJson(): String = this
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
}

enum class BiometricStatus {
    AVAILABLE,
    NO_HARDWARE,
    UNAVAILABLE,
    NOT_ENROLLED
}

sealed class BiometricResult {
    data object Success : BiometricResult()
    data class Error(val code: Int, val message: String) : BiometricResult()
}

data class EncryptedData(
    val data: ByteArray,
    val iv: ByteArray
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (javaClass != other?.javaClass) return false
        other as EncryptedData
        if (!data.contentEquals(other.data)) return false
        if (!iv.contentEquals(other.iv)) return false
        return true
    }
    
    override fun hashCode(): Int {
        var result = data.contentHashCode()
        result = 31 * result + iv.contentHashCode()
        return result
    }
}

data class ChatMessage(
    val id: String,
    val role: String,
    val content: String,
    val timestamp: Long
)
