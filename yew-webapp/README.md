## 📦 Installation & Setup

To run project locally, ensure you have the necessary dependencies installed.

### ✅ Prerequisites

Before starting, install the following:
- [Rust & Cargo](https://www.rust-lang.org/tools/install) 🦀
- [Trunk](https://trunkrs.dev/#install) 🚀
- [WebAssembly Target](https://rustwasm.github.io/wasm-pack/installer/) 🕸️
- [TailwindCSS - V4](https://tailwindcss.com/docs/installation) 🦶

### 🔥 Run the Dev Server

To start the **Dev Server** from the workspace directory, execute:

```sh
trunk serve --config yew-webapp/Trunk.toml
```
App will be available at http://localhost:8080

#### TailwindCSS

The `Trunk.toml` file is used to configure the build process, including the `tailwindcss` command.

If you use a custom `tailwindcss` command, you can modify the `Trunk.toml` file accordingly.


