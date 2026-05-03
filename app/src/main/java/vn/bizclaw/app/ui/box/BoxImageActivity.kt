package vn.bizclaw.app.ui.box

import android.graphics.BitmapFactory
import android.os.Bundle
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.launch
import vn.bizclaw.app.R
import vn.bizclaw.app.box.BoxConfig
import vn.bizclaw.app.box.BoxEngine
import java.io.File

/**
 * Box Image Activity - Generate images với AI
 */
class BoxImageActivity : AppCompatActivity() {
    
    private lateinit var boxEngine: BoxEngine
    
    private lateinit var promptInput: EditText
    private lateinit var negativePromptInput: EditText
    private lateinit var generateButton: Button
    private lateinit var progressBar: ProgressBar
    private lateinit var generatedImage: ImageView
    private lateinit var widthSlider: SeekBar
    private lateinit var heightSlider: SeekBar
    private lateinit var stepsSlider: SeekBar
    private lateinit var widthValue: TextView
    private lateinit var heightValue: TextView
    private lateinit var stepsValue: TextView
    
    private var width = 512
    private var height = 512
    private var steps = 20
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_box_image)
        
        initViews()
        initBoxEngine()
    }
    
    private fun initViews() {
        promptInput = findViewById(R.id.promptInput)
        negativePromptInput = findViewById(R.id.negativePromptInput)
        generateButton = findViewById(R.id.generateButton)
        progressBar = findViewById(R.id.progressBar)
        generatedImage = findViewById(R.id.generatedImage)
        
        widthSlider = findViewById(R.id.widthSlider)
        heightSlider = findViewById(R.id.heightSlider)
        stepsSlider = findViewById(R.id.stepsSlider)
        widthValue = findViewById(R.id.widthValue)
        heightValue = findViewById(R.id.heightValue)
        stepsValue = findViewById(R.id.stepsValue)
        
        // Width slider (256-1024)
        widthSlider.max = 768  // 1024 - 256
        widthSlider.progress = 256  // 512 - 256
        widthSlider.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(s: SeekBar?, progress: Int, fromUser: Boolean) {
                width = 256 + progress
                widthValue.text = "${width}px"
            }
            override fun onStartTrackingTouch(s: SeekBar?) {}
            override fun onStopTrackingTouch(s: SeekBar?) {}
        })
        widthValue.text = "${width}px"
        
        // Height slider
        heightSlider.max = 768
        heightSlider.progress = 256
        heightSlider.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(s: SeekBar?, progress: Int, fromUser: Boolean) {
                height = 256 + progress
                heightValue.text = "${height}px"
            }
            override fun onStartTrackingTouch(s: SeekBar?) {}
            override fun onStopTrackingTouch(s: SeekBar?) {}
        })
        heightValue.text = "${height}px"
        
        // Steps slider (10-50)
        stepsSlider.max = 40
        stepsSlider.progress = 10
        stepsSlider.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(s: SeekBar?, progress: Int, fromUser: Boolean) {
                steps = 10 + progress
                stepsValue.text = steps.toString()
            }
            override fun onStartTrackingTouch(s: SeekBar?) {}
            override fun onStopTrackingTouch(s: SeekBar?) {}
        })
        stepsValue.text = steps.toString()
        
        generateButton.setOnClickListener { generateImage() }
        generatedImage.visibility = View.GONE
    }
    
    private fun initBoxEngine() {
        boxEngine = BoxEngine(this)
        
        lifecycleScope.launch {
            val config = BoxConfig(
                modelPath = getExternalFilesDir("models")?.absolutePath + "/hermes-2-pro-q4.gguf",
                imageEnabled = true,
                imagePath = getExternalFilesDir("models")?.absolutePath + "/sd-v1-5.gguf"
            )
            
            boxEngine.initialize(config).onSuccess {
                runOnUiThread {
                    generateButton.isEnabled = true
                }
            }.onFailure {
                runOnUiThread {
                    Toast.makeText(
                        this@BoxImageActivity,
                        "Image model not loaded",
                        Toast.LENGTH_SHORT
                    ).show()
                }
            }
        }
    }
    
    private fun generateImage() {
        val prompt = promptInput.text.toString().trim()
        if (prompt.isEmpty()) {
            Toast.makeText(this, "Please enter a prompt", Toast.LENGTH_SHORT).show()
            return
        }
        
        val negativePrompt = negativePromptInput.text.toString().trim()
        
        generateButton.isEnabled = false
        progressBar.visibility = View.VISIBLE
        progressBar.isIndeterminate = true
        
        lifecycleScope.launch {
            val result = boxEngine.generateImage(
                prompt = prompt,
                negativePrompt = negativePrompt,
                width = width,
                height = height,
                steps = steps
            )
            
            runOnUiThread {
                progressBar.visibility = View.GONE
                generateButton.isEnabled = true
                
                result.onSuccess { imagePath ->
                    val bitmap = BitmapFactory.decodeFile(imagePath)
                    generatedImage.setImageBitmap(bitmap)
                    generatedImage.visibility = View.VISIBLE
                    
                    // Save button
                    findViewById<Button>(R.id.saveButton).apply {
                        visibility = View.VISIBLE
                        setOnClickListener {
                            saveImage(imagePath)
                        }
                    }
                    
                }.onFailure { e ->
                    Toast.makeText(
                        this@BoxImageActivity,
                        "Error: ${e.message}",
                        Toast.LENGTH_LONG
                    ).show()
                }
            }
        }
    }
    
    private fun saveImage(imagePath: String) {
        try {
            val destFile = File(
                getExternalFilesDir(null),
                "generated_${System.currentTimeMillis()}.png"
            )
            File(imagePath).copyTo(destFile, overwrite = true)
            
            Toast.makeText(this, "Saved: ${destFile.absolutePath}", Toast.LENGTH_LONG).show()
        } catch (e: Exception) {
            Toast.makeText(this, "Save failed: ${e.message}", Toast.LENGTH_SHORT).show()
        }
    }
    
    override fun onDestroy() {
        super.onDestroy()
        boxEngine.release()
    }
}
