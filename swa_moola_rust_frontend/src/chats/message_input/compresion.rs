use futures_channel::oneshot;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::{HtmlCanvasElement, HtmlImageElement, Url, Blob, AudioContext, AudioBuffer};

pub async fn convert_to_compressed_audio_bytes(file: web_sys::File) -> Result<Vec<u8>, wasm_bindgen::JsValue> {
   
    // 1. Read the audio file into an ArrayBuffer
    let array_buffer_promise = file.array_buffer();
    let array_buffer = JsFuture::from(array_buffer_promise).await?;
    
    // 2. Create an AudioContext to process the sound
    let audio_ctx = AudioContext::new()?;
    
    // 3. Decode the audio data (This instantly drops all metadata blocks)
    let decode_promise = audio_ctx.decode_audio_data(&array_buffer.dyn_into::<js_sys::ArrayBuffer>()?)?;
    let audio_buffer = JsFuture::from(decode_promise).await?.dyn_into::<AudioBuffer>()?;
    
    // 4. Extract raw channel data (Mono channel to save 50% more space)
    let js_mono_data = audio_buffer.get_channel_data(0)?;
    let mono_data: Vec<f32> = js_mono_data.to_vec();
    
    // 5. Convert the raw floating-point data into compact 16-bit WAV bytes
    let compressed_bytes = create_wav_bytes(&mono_data, audio_buffer.sample_rate());
    
    Ok(compressed_bytes)
}

fn create_wav_bytes(samples: &[f32], sample_rate: f32) -> Vec<u8> {
    let mut buffer = Vec::new();
    
    let num_samples = samples.len();
    let num_channels: u16 = 1; // Mono channel
    let bits_per_sample: u16 = 16; // 16-bit PCM quality
    let sample_rate_u32 = sample_rate as u32;
    
    let byte_rate = sample_rate_u32 * (num_channels as u32) * (bits_per_sample as u32 / 8);
    let block_align = num_channels * (bits_per_sample / 8);
    let sub_chunk_2_size = (num_samples * (bits_per_sample as usize / 8)) as u32;
    let chunk_size = 36 + sub_chunk_2_size;

    // --- 1. WRITE THE 44-BYTE WAV HEADER ---
    buffer.extend_from_slice(b"RIFF");                     // ChunkID
    buffer.extend_from_slice(&chunk_size.to_le_bytes());   // ChunkSize
    buffer.extend_from_slice(b"WAVE");                     // Format
    
    buffer.extend_from_slice(b"fmt ");                     // Subchunk1ID
    buffer.extend_from_slice(&16u32.to_le_bytes());        // Subchunk1Size (16 for PCM)
    buffer.extend_from_slice(&1u16.to_le_bytes());         // AudioFormat (1 for uncompressed PCM)
    buffer.extend_from_slice(&num_channels.to_le_bytes()); // NumChannels
    buffer.extend_from_slice(&sample_rate_u32.to_le_bytes()); // SampleRate
    buffer.extend_from_slice(&byte_rate.to_le_bytes());    // ByteRate
    buffer.extend_from_slice(&block_align.to_le_bytes());  // BlockAlign
    buffer.extend_from_slice(&bits_per_sample.to_le_bytes()); // BitsPerSample

    buffer.extend_from_slice(b"data");                     // Subchunk2ID
    buffer.extend_from_slice(&sub_chunk_2_size.to_le_bytes()); // Subchunk2Size

    // --- 2. CONVERT SAMPLES TO 16-BIT PCM BYTES ---
    for &sample in samples {
        // Clamp values between -1.0 and 1.0 to prevent audio distortion
        let clamped = sample.clamp(-1.0, 1.0);
        
        // Scale the 32-bit floating point number to a signed 16-bit integer
        let scaled = if clamped < 0.0 {
            (clamped * 32768.0) as i16
        } else {
            (clamped * 32767.0) as i16
        };
        
        // Write the 2 bytes of the 16-bit integer into our buffer
        buffer.extend_from_slice(&scaled.to_le_bytes());
    }

    buffer
}

pub async fn convert_to_clean_webp_bytes(file: web_sys::File) -> Result<Vec<u8>, wasm_bindgen::JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window found"))?;
    let object_url = Url::create_object_url_with_blob(&file)?;
    let img = window.document().unwrap().create_element("img")?.dyn_into::<HtmlImageElement>()?;
    img.set_src(&object_url);

    // 1. Wait for the browser to finish loading the image
    let (tx, rx) = oneshot::channel::<()>();
    let mut tx : Option<oneshot::Sender<()>> = Some(tx);
    let closure = Closure::once(move || { 
        if let Some(t) = tx.take() { 
            let _ = t.send(()); 
        } 
    });
    img.set_onload(Some(closure.as_ref().unchecked_ref()));
    let _ = rx.await; 

    // 2. Paint to a hidden canvas (This step instantly deletes hidden metadata blocks)
    let canvas = window.document().unwrap().create_element("canvas")?.dyn_into::<HtmlCanvasElement>()?;
    canvas.set_width(img.natural_width());
    canvas.set_height(img.natural_height());
    canvas.get_context("2d")?.unwrap().dyn_into::<web_sys::CanvasRenderingContext2d>()?.draw_image_with_html_image_element(&img, 0.0, 0.0)?;
    Url::revoke_object_url(&object_url)?; 

    // 3. Compress to small WebP bytes
    let (tx_blob, rx_blob) = oneshot::channel::<JsValue>();
    let mut tx_blob: Option<oneshot::Sender<JsValue>> = Some(tx_blob);
    
    // Changed parameter type from leptos::prelude::JsValue to wasm_bindgen::JsValue
    let blob_closure = Closure::once(move |b: wasm_bindgen::JsValue| { 
        if let Some(t) = tx_blob.take() { 
            let _ = t.send(b); 
        } 
    });
    
    canvas.to_blob_with_type_and_encoder_options(
        blob_closure.as_ref().unchecked_ref(), 
        "image/webp", 
        &wasm_bindgen::JsValue::from_f64(0.80)
    )?;
    
    // 4. Convert the resulting blob into a Rust Vec<u8>
    let blob_js_value = rx_blob.await.map_err(|_| JsValue::from_str("Channel closed"))?;
    let blob = blob_js_value.dyn_into::<Blob>()?;
    let array_buffer = JsFuture::from(blob.array_buffer()).await?;
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    
    Ok(uint8_array.to_vec())
}
