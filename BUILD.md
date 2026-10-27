# Guia de Build e Infraestrutura de CI/CD - Guerra Fria (1945+)

Este documento detalha o fluxo de compilação, desenvolvimento rápido, empacotamento, cross-compilação e automação de CI/CD para o jogo **Guerra Fria** (Rust + Bevy).

---

## 1. Visão Geral da Arquitetura do Workspace

O projeto é um Workspace Cargo modular (`crates/`):

| Crate | Tipo | Descrição |
| :--- | :--- | :--- |
| `guerra_fria` | Binário | Cliente do jogo em Rust + Bevy + `bevy_egui` (renderização, UI, loop interativo). Único crate que exige GPU/janela. |
| `gerador_mapa` | Binário (CLI) | Ferramenta que gera e valida os dados geográficos e demográficos de 1945 (RON). |
| `simulacao` | Biblioteca | Motor determinístico legado, sem Bevy. |
| `sim_economia`, `sim_politica`, `sim_territorio`, `sim_guerra` | Bibliotecas | Simuladores puros por domínio, sem Bevy; testáveis sem GPU. |
| `bandeiras_resolver` | Biblioteca | Resolução de bandeiras (variantes por país/regime). |

Os testes rodam com `cargo test --workspace --exclude guerra_fria` (sem GPU). Os pacotes de release incluem `assets/` e `data/`.

---

## 2. Pré-requisitos do Sistema

### 2.1. Rust Toolchain
Recomenda-se a versão estável mais recente do Rust (1.85+):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
```

### 2.2. Dependências do Sistema no Linux (para Bevy)
O Bevy depende de bibliotecas nativas de áudio, janelas e entrada do sistema operacional.

#### Ubuntu / Debian / Pop!_OS:
```bash
sudo apt-get update
sudo apt-get install --no-install-recommends -y \
  pkg-config \
  libasound2-dev \
  libudev-dev \
  libwayland-dev \
  libxkbcommon-dev
```

#### Arch Linux / Manjaro:
```bash
sudo pacman -S --needed \
  pkgconf \
  alsa-lib \
  systemd-libs \
  wayland \
  libxkbcommon
```

#### Fedora / RHEL:
```bash
sudo dnf install -y \
  pkgconf-pkg-config \
  alsa-lib-devel \
  systemd-devel \
  wayland-devel \
  libxkbcommon-devel
```

### 2.3. Pré-requisitos no Windows
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) com o workload **"Desktop development with C++"** (incluindo Windows SDK).
- Rust configurado com o target padrão `x86_64-pc-windows-msvc`.

---

## 3. Otimizações de Compilação para Bevy

A compilação de jogos em Bevy pode ser pesada se perfis incorretos forem utilizados. Este repositório vem pré-configurado com as melhores práticas da comunidade:

### 3.1. Perfis de Compilação (definidos no `Cargo.toml`)
- **Modo Desenvolvimento (`profile.dev`)**:
  - `opt-level = 1` no nosso próprio código: compila rapidamente enquanto aplica otimizações básicas.
  - `opt-level = 3` nas dependências (`[profile.dev.package."*"]`): garante que Bevy, wgpu e matemática rodem em 60 FPS durante o desenvolvimento, sem lentidão extrema de opt-level 0.
- **Modo Release (`profile.release`)**:
  - `opt-level = 3` e `lto = "thin"`: otimização inter-módulos com tamanho reduzido.
  - `codegen-units = 1` e `panic = "abort"`: binário final compacto e sem overhead desnecessário de unwinding.
  - `strip = true`: remove automaticamente símbolos de depuração da release final.

### 3.2. Linkers Rápidos (`mold` e `lld`) via `.cargo/config.toml`
Em projetos Bevy, a etapa de linkagem com o linker tradicional do GNU pode demorar de 5 a 15 segundos a cada salvamento. O uso de linkers modernos reduz isso para milissegundos:

1. **mold** (Linux - mais rápido):
   - Instale: `sudo apt install mold clang` (Ubuntu) ou `sudo pacman -S mold clang` (Arch).
   - Descomente as linhas correspondentes em `.cargo/config.toml`:
     ```toml
     [target.x86_64-unknown-linux-gnu]
     linker = "clang"
     rustflags = ["-C", "link-arg=-fuse-ld=mold"]
     ```

2. **LLD** (Alternativa LLVM universal):
   - Instale: `sudo apt install lld clang` (Ubuntu) ou `sudo pacman -S lld clang` (Arch).
   - Descomente em `.cargo/config.toml`:
     ```toml
     [target.x86_64-unknown-linux-gnu]
     linker = "clang"
     rustflags = ["-C", "link-arg=-fuse-ld=lld"]
     ```

### 3.3. Compilação com Linkagem Dinâmica (`dynamic_linking`)
Durante o ciclo de desenvolvimento diário de UI e sistemas, ative a feature `dynamic_linking` para compilar o motor Bevy como biblioteca compartilhada (.so / .dll):
```bash
cargo run --package guerra_fria --features dynamic_linking
# ou usando Makefile:
make run-fast
```
> **Nota:** Não utilize a feature `dynamic_linking` para builds finais de release destinadas a distribuição.

---

## 4. Cross-Compilação Linux -> Windows

Desenvolvedores no Linux podem compilar e empacotar binários `.exe` para Windows de duas formas suportadas:

### Opção 1: `cargo-xwin` (Target: `x86_64-pc-windows-msvc`) - **Recomendada**
Produz binários nativos com a ABI oficial da Microsoft (MSVC), compatível com DirectX 12, PDBs de depuração e drivers de vídeo oficiais.
- **Como instalar**:
  ```bash
  cargo install --locked cargo-xwin
  rustup target add x86_64-pc-windows-msvc
  ```
- **Como executar manualmente**:
  ```bash
  cargo xwin build --target x86_64-pc-windows-msvc --release --workspace
  ```

### Opção 2: MinGW-w64 (Target: `x86_64-pc-windows-gnu`)
Utiliza o compilador cruzado GNU MinGW empacotado na própria distribuição Linux.
- **Como instalar**:
  - Ubuntu/Debian: `sudo apt-get install -y mingw-w64`
  - Arch Linux: `sudo pacman -S mingw-w64-gcc`
  - Adicionar target: `rustup target add x86_64-pc-windows-gnu`
- **Configuração no `.cargo/config.toml`**:
  Já está configurada no projeto:
  ```toml
  [target.x86_64-pc-windows-gnu]
  linker = "x86_64-w64-mingw32-gcc"
  ar = "x86_64-w64-mingw32-ar"
  ```
- **Como executar manualmente**:
  ```bash
  cargo build --target x86_64-pc-windows-gnu --release --workspace
  ```

### Script Automatizado (`scripts/build_windows_cross.sh`)
O script fornecido automatiza a compilação e gera diretamente o pacote `.zip` com a pasta `assets`:
```bash
# Cross-compilação com cargo-xwin (padrão)
./scripts/build_windows_cross.sh --xwin --release

# Cross-compilação com MinGW
./scripts/build_windows_cross.sh --mingw --release

# Apenas compilar em debug sem gerar zip
./scripts/build_windows_cross.sh --debug --no-package
```

---

## 5. Automação com Makefile e justfile

Para conveniência da equipe, estão disponíveis tanto um `Makefile` tradicional quanto um `justfile` moderno:

| Ação | Comando `make` | Comando `just` |
| :--- | :--- | :--- |
| Exibir ajuda com comandos | `make help` | `just` |
| Verificar tipagem e sintaxe | `make check` | `just check` |
| Formatar código-fonte | `make fmt` | `just fmt` |
| Validar formatação | `make fmt-check` | `just fmt-check` |
| Análise estática com Clippy | `make clippy` | `just clippy` |
| Executar testes unitários | `make test` | `just test` |
| Compilar (debug) | `make build` | `just build` |
| Compilar (release) | `make build-release` | `just build-release` |
| Executar o jogo | `make run` | `just run` |
| Executar com dynamic linking | `make run-fast` | `just run-fast` |
| Executar ferramenta de mapa | `make run-map-gen` | `just run-map-gen` |
| Cross-compile Windows (xwin) | `make cross-windows-xwin`| `just cross-windows-xwin` |
| Cross-compile Windows (mingw)| `make cross-windows-mingw`| `just cross-windows-mingw` |
| Gerar pacote Linux (`.tar.gz`) | `make package-linux` | `just package-linux` |
| Gerar pacote Windows (`.zip`) | `make package-windows` | `just package-windows` |
| Validar workflows de CI | `make validate-ci` | `just validate-ci` |
| Limpar artefatos | `make clean` | `just clean` |

---

## 6. Pipeline de CI/CD (GitHub Actions)

A automação do repositório no GitHub é gerenciada por dois workflows em `.github/workflows/`:

### 6.1. Integração Contínua (`.github/workflows/ci.yml`)
Disparado a cada `push` ou `pull_request` nos branches `main`, `master` e `develop`.
- **Jobs**:
  1. `fmt`: Verifica formatação de código com `cargo fmt --all --check` no Ubuntu.
  2. `clippy`: Executa `cargo clippy --workspace --all-targets -- -D warnings` tratando alertas como erros.
  3. `test`: Executa `cargo test --workspace --exclude guerra_fria` (sem GPU) em matriz paralela (`ubuntu-latest` e `windows-latest`).
  4. `build`: Garante a compilação completa de todos os alvos e crates em `ubuntu-latest` e `windows-latest`.
- **Recursos**:
  - Instalação automática das dependências do sistema Linux para Bevy (`libasound2-dev`, `libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`, `pkg-config`).
  - Cache inteligente de crates e artefatos intermediários com `Swatinem/rust-cache@v2`.
  - Cancelamento automático de builds anteriores obsoletos com `concurrency.cancel-in-progress`.

### 6.2. Publicação de Releases (`.github/workflows/release.yml`)
Disparado ao criar uma tag iniciada com `v*` (exemplo: `v0.1.0`):
- **Job `build-release`**:
  - Compila `guerra_fria` e `gerador_mapa` em modo release para Linux (`x86_64-unknown-linux-gnu`) e Windows (`x86_64-pc-windows-msvc`).
  - Inclui automaticamente a pasta `assets`, `data/`, executáveis (`guerra_fria` e `gerador_mapa`) e arquivos `README.md` / `BUILD.md`.
  - Gera os arquivos compactados com a tag/versão explícita no nome:
    - `guerra-fria-v0.1.0-linux-x86_64.tar.gz`
    - `guerra-fria-v0.1.0-windows-x86_64.zip`
- **Job `publish-release`**:
  - Baixa os artefatos de ambas as plataformas.
  - Cria um **GitHub Release** oficial utilizando `softprops/action-gh-release@v2`.
  - Anexa os arquivos `.tar.gz` e `.zip` e gera notas de release automaticamente baseadas nos commits.

---

## 7. Estrutura dos Pacotes Distribuíveis

Os arquivos compactados finais gerados para os usuários possuem a seguinte estrutura interna:

```
guerra-fria-v0.1.0-linux-x86_64.tar.gz (ou .zip no Windows)
├── guerra_fria (ou guerra_fria.exe)
├── gerador_mapa (ou gerador_mapa.exe)
├── assets/
│   ├── map/
│   └── shaders/
├── data/
│   └── scenario_1945/
├── README.md
└── BUILD.md
```

Ao descompactar em qualquer máquina (Linux ou Windows), o executável encontra a pasta `assets/` no mesmo diretório de execução, garantindo que o carregamento de recursos do Bevy funcione perfeitamente sem necessidade de instalação global.
