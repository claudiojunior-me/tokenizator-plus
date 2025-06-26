# Tokenizator-Plus

A simple yet powerful web utility built in Rust to analyze local codebases. It provides a file tree view, concatenated file content with line numbers, and an estimated token count, making it easy to prepare large code contexts for analysis or use with Large Language Models (LLMs).

The entire application runs as a single, self-contained binary and is fully containerized with Docker for easy deployment and development.

---

## ✨ Features

-   **Recursive Directory Analysis**: Provide a path to a local project, and the tool recursively scans its structure.
-   **File Tree and Content View**: Displays a clean file tree followed by the full content of each file, complete with line numbers for easy reference.
-   **Dynamic Ignore Patterns**: Use glob patterns (e.g., `node_modules`, `*.log`, `target/**`) to exclude specific files and directories from the analysis in real-time.
-   **Token Counting**: Uses the `tiktoken-rs` implementation of OpenAI's `cl100k_base` tokenizer to precisely measure tokens for the concatenated output.
-   **Custom Token Limit**: Specify a maximum number of tokens to generate (default `150000`) to keep the output manageable.
-   **Fully Dockerized**: Includes separate, optimized Dockerfiles for development (with hot-reloading) and production (with a minimal final image).
-   **Zero Frontend Dependencies**: The UI is built with pure, dependency-free HTML, CSS, and vanilla JavaScript.

## 🛠️ Tech Stack

-   **Backend**:
    -   [Rust](https://www.rust-lang.org/)
    -   [Actix Web](https://actix.rs/): High-performance web framework.
    -   [WalkDir](https://crates.io/crates/walkdir): For recursive directory traversal.
    -   [Glob](https://crates.io/crates/glob): For matching ignore patterns.
    -   [Tera](https://tera.netlify.app/): Templating engine for HTML rendering.
    -   [tiktoken-rs](https://crates.io/crates/tiktoken-rs): Accurate tokenization for OpenAI models.
-   **Frontend**:
    -   HTML5
    -   Pure CSS3
    -   Vanilla JavaScript
-   **Containerization**:
    -   [Docker](https://www.docker.com/) & [Docker Compose](https://docs.docker.com/compose/)

## 🚀 Getting Started

### Prerequisites

-   [Rust](https://www.rust-lang.org/tools/install) (if running locally without Docker)
-   [Docker](https://docs.docker.com/get-docker/) & [Docker Compose](https://docs.docker.com/compose/install/)

### Installation & Usage

There are two primary ways to run the application:

#### 1. Via Docker (Recommended)

This is the easiest and most consistent way to run the project.

**For Development (with Hot-Reloading):**

The `docker-compose.yml` file is configured for a smooth development experience.

1.  Clone the repository.
2.  (Optional) In `docker-compose.yml`, change the volume mount `- ./data:/data` to point to a project you want to analyze by default, for example: `- ~/my-projects/cool-project:/data`.
3.  Run the following command from the project root:
    ```bash
    docker-compose up --build
    ```
4.  The application will be available at `http://localhost:3000`. The Rust backend will automatically recompile and restart whenever you save a `.rs` file.

**For Production:**

This method builds a minimal, optimized production image.

1.  Build the production image:
    ```bash
    docker build -f Dockerfile.prod -t tokenizator-plus:latest .
    ```

2.  Run the container. You must specify which host directory you want to analyze by mapping it to the `/data` volume inside the container.

    For example, to analyze the project located at `/home/user/my-app` on your machine:
    ```bash
    docker run -p 3000:3000 \
           -v /home/user/my-app:/data \
           -e DATA_DIR_BASE=/data \
           tokenizator-plus:latest
    ```
    Then, in the web UI, you would enter `/` or `./` in the path input to analyze the root of `my-app`.

#### 2. Locally (Without Docker)

This is ideal for quick tests or if you prefer not to use Docker.

1.  Clone the repository.
2.  Run the application with Cargo:
    ```bash
    cargo run
    ```
3.  The application will be available at `http://localhost:3000`.
4.  Since the app is running directly on your host, you can provide any absolute path (e.g., `/home/user/my-app`) or a path relative to the `tokenizator-plus` directory in the web UI.

## ⚙️ How to Use the Interface

1.  **Path Input**: Enter the path of the directory you want to analyze.
    -   When running via Docker, this path is *relative to the `/data` volume* you mounted (e.g., `/` or `/src`).
    -   When running locally, this can be any absolute or relative path on your system.
2.  **Ignore Patterns**: Type a glob pattern (like `target`, `*.md`, `dist/**`) into the ignore input and press Enter. It will be added as a tag. You can add multiple patterns. Click the `×` on a tag to remove it.
3.  **Max Tokens**: (Optional) Enter the maximum number of tokens to output before the process stops. The default is `150000`.
4.  **Analyze**: Click the "Analyze" button to start the process.
5.  **Output**: The file tree and concatenated content will appear in the main text area. You can use the "Copy All" button to copy the entire output to your clipboard.

## License

This project is licensed under the MIT License.