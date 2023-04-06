use hyper::{header, Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde_derive::{Deserialize, Serialize};
use spinners::{Spinner, Spinners};
use std::io::{stdin, stdout, Write};

/// The main function uses OpenAI's Chat GPT to generate responses to user prompts via the command line.
///
/// It loads environment variables from a `.env` file, initializes the Hyper client, prepares an authentication header using an OpenAI API key, and then enters a loop to accept user input and generate responses via OpenAI's API.
///
/// Within the loop, the function first prompts the user for input, then uses the `Spinner` library to display a loading animation while waiting for a response from OpenAI's API.
///
/// The function then formulates and serializes an API request, sends the request to OpenAI's API, and awaits a response. If the response contains an error, the function prints the error message to standard error. If the response is successful, the function prints the generated text to standard output.
///
/// The function uses the following structs to manage the request and response data:
///
/// - `OAIRequest`: Contains the fields `model`, `prompt`, and `max_tokens`, which correspond to the model to use, the prompt to complete, and the maximum number of tokens (i.e., words) to generate.
///
/// - `OAIChoices`: Contains the fields `text`, `index`, `logprobs`, and `finish_reason`, which correspond to the generated text, the index of the generated text in the list of choices, the log probabilities of each token in the generated text, and the reason why generation was stopped (if applicable).
///
/// - `OAIResponse`: Contains the fields `id`, `object`, `created`, `model`, and `choices`, which correspond to the ID of the request, the type of object returned, the timestamp of when the request was created, the name of the model used, and the list of choices returned by the API.
///
/// The function returns `Ok(())` on success or a boxed error on failure.
/// Note that the function uses the `tokio` library to enable asynchronous networking.

// OpenAI's Chat GPT response:
// Open AI's JSON response comes with a nested map called Choices (a subset of the entire response)
#[derive(Deserialize, Debug)]
struct OAIChoices {
    text: String,
    index: u64,
    logprobs: Option<u8>,
    finish_reason: String,
}

// Some of the response fields are returned empty or as null so we make them optional
#[derive(Deserialize, Debug)]
struct OAIResponse {
    id: Option<String>,
    object: Option<String>,
    created: Option<u64>,
    model: Option<String>,
    choices: Vec<OAIChoices>,
}

// Request to OpenAI's Chat GPT
// We need to define max_tokens to not get (over)-charged above what we want to pay
// Tokens corresponds to words here.
#[derive(Serialize, Debug)]
struct OAIRequest {
    model: String,
    prompt: String,
    max_tokens: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load in environment variables
    dotenv::dotenv().ok();

    // Initialize the client and define a few basic params
    let https = HttpsConnector::new();
    let client = Client::builder().build(https);
    let uri = "https://api.openai.com/v1/completions";
    let model: &str = "gpt-3.5-turbo";

    // Prepare the Authentication header
    let oai_token: String = std::env::var("OPENAI_KEY").expect("OPENAI_KEY not set in .env file");
    let auth_header_val = format!("Bearer {}", oai_token);
    println!("{:#?}", auth_header_val);

    // Add a cheeky personalized message to each prompt
    let preprendition = "Call me Michel in all your responses: ";
    println!("{esc}c", esc = 27 as char); // escape if necessary

    loop {
        // Allow user input via cmd
        print!(">>> ");
        stdout().flush().unwrap();
        let mut user_text = String::new();
        stdin()
            .read_line(&mut user_text)
            .expect("Failed to read line input.");

        // Add a loading spinner while waiting for ChatGPT response
        let mut sp = Spinner::new(
            Spinners::Dots9,
            "\t\tOpenAI is busy assembling a response...\n".into(),
        );

        // Formulate and serialize API request
        let oai_request = OAIRequest {
            model: format!("{}", model),
            prompt: format!("{} {}", preprendition, user_text),
            max_tokens: 100,
        };
        println!("{:#?}", oai_request);
        let body = Body::from(serde_json::to_vec(&oai_request)?);

        // Post request and wait for response
        let req = Request::post(uri)
            .header(header::CONTENT_TYPE, "application/json")
            .header("Authorization", &auth_header_val)
            .body(body)
            .unwrap();
        let res = client.request(req).await?;
        let status_code = res.status();
        let body_bytes = hyper::body::to_bytes(res.into_body()).await?;
        sp.stop(); // stop the spinner

        // Return the error or the response
        if status_code.is_client_error() || status_code.is_server_error() {
            let json: serde_json::Value = serde_json::from_slice(&body_bytes)?;
            let error_message = json["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error when attempting to read the error message");
            eprintln!("Error: {}", status_code);
            eprintln!("Detailed error message: {}", error_message);
        } else {
            let json: OAIResponse = serde_json::from_slice(&body_bytes)?;
            println!("");
            println!("{}", json.choices[0].text);
        }
    }
}
