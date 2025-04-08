use serde::{Deserialize, Serialize};
use tauri::command;
use std::env;

// Structures based on Gemini API's generateContent method (common pattern)
#[derive(Serialize, Debug)]
struct GeminiRequest {
    contents: Vec<Content>,
    // We could add generationConfig here if needed
}

#[derive(Serialize, Debug)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize, Debug)]
struct Part {
    text: String,
}

// --- Gemini API Response Structures ---
#[derive(Deserialize, Debug)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
    // promptFeedback might also be present
}

#[derive(Deserialize, Debug)]
struct Candidate {
    content: ContentResponse,
    // finishReason, index, safetyRatings might also be present
}

#[derive(Deserialize, Debug)]
struct ContentResponse {
    parts: Vec<PartResponse>,
    // role might also be present (usually "model")
}

#[derive(Deserialize, Debug)]
struct PartResponse {
    text: String,
}

// Tauri command to send a prompt to Gemini
#[command]
pub async fn send_to_gemini(prompt: String) -> Result<String, String> {
    let api_key = env::var("GEMINI_API_KEY").map_err(|e| format!("Failed to get GEMINI_API_KEY: {}", e))?;
    // Use the specific model in the endpoint URL. Adjust region/project if needed.
    // Using v1beta as an example, check current Gemini docs for stable endpoints.
    let model_name = "gemini-2.0-flash"; // Updated model name
    let api_endpoint = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name, api_key
    );

    let client = reqwest::Client::new();

    // Construct the request body according to the Gemini API structure
    let request_body = GeminiRequest {
        contents: vec![
            Content {
                parts: vec![ Part { text: prompt } ]
            }
        ]
    };

   // TODO: Replace YOUR_GEMINI_API_ENDPOINT with the actual endpoint.
   // TODO: Ensure GEMINI_API_KEY environment variable is set.
   // TODO: Adjust GeminiRequest and GeminiResponse structs for the specific API endpoint.

   // The API key is now in the URL, so we don't need separate auth headers typically.
   // If using a different auth method (e.g., OAuth), adjust accordingly.
   let response = client.post(&api_endpoint) // Pass endpoint by reference
       .json(&request_body)
       .send()
       .await
       .map_err(|e| format!("API request failed: {}", e))?;

   if response.status().is_success() {
       let gemini_response = response.json::<GeminiResponse>().await
           .map_err(|e| format!("Failed to parse API response: {}", e))?;

       // Extract text from the structured response
       // Handle potential missing fields gracefully
       let response_text = gemini_response.candidates.get(0)
           .and_then(|candidate| candidate.content.parts.get(0))
           .map(|part| part.text.clone())
           .unwrap_or_else(|| "No text content found in response".to_string());

       Ok(response_text)
   } else {
       let status = response.status();
       let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
       Err(format!("API request failed with status {}: {}", status, error_text))
   }
}

// Add other necessary functions, e.g., for managing conversation history, etc.