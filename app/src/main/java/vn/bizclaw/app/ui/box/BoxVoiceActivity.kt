package vn.bizclaw.app.ui.box

import android.Manifest
import android.content.pm.PackageManager
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.Bundle
import android.widget.Button
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vn.bizclaw.app.R
import vn.bizclaw.app.box.BoxConfig
import vn.bizclaw.app.box.BoxEngine

class BoxVoiceActivity : AppCompatActivity() {
    
    private lateinit var boxEngine: BoxEngine
    private lateinit var recordButton: Button
    private lateinit var statusText: TextView
    private lateinit var transcriptText: TextView
    
    private var isRecording = false
    private var audioRecord: AudioRecord? = null
    private val sampleRate = 16000
    private val channelConfig = AudioFormat.CHANNEL_IN_MONO
    private val audioFormat = AudioFormat.ENCODING_PCM_16BIT
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_box_voice)
        
        initViews()
        initBoxEngine()
    }
    
    private fun initViews() {
        recordButton = findViewById(R.id.recordButton)
        statusText = findViewById(R.id.statusText)
        transcriptText = findViewById(R.id.transcriptText)
        
        recordButton.setOnClickListener { toggleRecording() }
        
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) 
            != PackageManager.PERMISSION_GRANTED) {
            ActivityCompat.requestPermissions(
                this,
                arrayOf(Manifest.permission.RECORD_AUDIO),
                1001
            )
        }
    }
    
    private fun initBoxEngine() {
        boxEngine = BoxEngine(this)
        
        lifecycleScope.launch {
            val config = BoxConfig(
                modelPath = filesDir.absolutePath + "/models/hermes-2-pro-q4.gguf",
                whisperPath = filesDir.absolutePath + "/models/whisper-tiny.bin"
            )
            
            boxEngine.initialize(config).onSuccess {
                runOnUiThread {
                    statusText.text = "Ready"
                }
            }.onFailure { e ->
                runOnUiThread {
                    statusText.text = "Error: ${e.message}"
                }
            }
        }
    }
    
    private fun toggleRecording() {
        if (isRecording) {
            stopRecording()
        } else {
            startRecording()
        }
    }
    
    private fun startRecording() {
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO) 
            != PackageManager.PERMISSION_GRANTED) {
            return
        }
        
        val bufferSize = AudioRecord.getMinBufferSize(sampleRate, channelConfig, audioFormat)
        
        try {
            audioRecord = AudioRecord(
                MediaRecorder.AudioSource.MIC,
                sampleRate,
                channelConfig,
                audioFormat,
                bufferSize
            )
            
            audioRecord?.startRecording()
            isRecording = true
            
            runOnUiThread {
                recordButton.text = "Stop"
                statusText.text = "Recording..."
            }
            
            lifecycleScope.launch(Dispatchers.IO) {
                val audioData = recordAudio()
                processAudio(audioData)
            }
        } catch (e: Exception) {
            runOnUiThread {
                Toast.makeText(this@BoxVoiceActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }
    
    private suspend fun recordAudio(): ByteArray {
        val bufferSize = AudioRecord.getMinBufferSize(sampleRate, channelConfig, audioFormat)
        val buffer = ByteArray(bufferSize * 10)
        var totalRead = 0
        
        while (isRecording && totalRead < buffer.size) {
            val read = audioRecord?.read(buffer, totalRead, bufferSize)
            if (read != null && read > 0) {
                totalRead += read
            }
        }
        
        return buffer.copyOf(totalRead)
    }
    
    private fun stopRecording() {
        isRecording = false
        audioRecord?.stop()
        audioRecord?.release()
        audioRecord = null
        
        runOnUiThread {
            recordButton.text = "Record"
            statusText.text = "Processing..."
        }
    }
    
    private fun processAudio(audioData: ByteArray) {
        lifecycleScope.launch {
            val transcription = withContext(Dispatchers.IO) {
                boxEngine.transcribe(audioData)
            }
            
            runOnUiThread {
                transcription.onSuccess { text ->
                    transcriptText.text = text
                    statusText.text = "Done"
                }.onFailure { e ->
                    statusText.text = "Error: ${e.message}"
                }
            }
        }
    }
    
    override fun onDestroy() {
        super.onDestroy()
        if (isRecording) stopRecording()
        boxEngine.release()
    }
}
