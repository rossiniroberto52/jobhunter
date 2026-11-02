# jobhunter

Bot em Rust que busca vagas, pontua pelo seu perfil e avisa no Telegram.
A candidatura final é **sempre manual**: o bot só te entrega as melhores vagas.

## Fontes de vagas

- **Remotive** — API pública de vagas remotas
- **Wellfound (AngelList)** — vagas de startup
- **RSS/Atom** — qualquer careers page que exponha feed (configure em `[rss]` no `config.toml`)

Para adicionar uma nova fonte, implemente o trait `Source` em `src/sources/` e registre em `all()`.

## Como usar

```bash
cargo build --release

# 1) busca, pontua e salva as vagas novas (rode no máximo algumas vezes por dia)
cargo run --release -- fetch

# 2) vê as melhores no terminal (TUI)
cargo run --release -- tui
#   j/k: navega | a: applied | i: interview | r: rejected | o: ignored | n: new
#   +/-: score | R: atualizar | q: sair

# 2b) ou abra a interface gráfica (GUI)
cargo run --release -- gui
#   Buscar vagas | Avisar Telegram | Atualizar lista | Importar currículo
#   Clique em "Abrir" para abrir no navegador
#   Botões Applied / Interview / Rejected / Ignored / New mudam o status
#   "Importar currículo" abre um seletor de arquivo (.txt/.md/.pdf), extrai keywords
#   e grava pesos sugeridos em pesos-sugeridos.toml
#   PDFs funcionam direto no binário (lopdf embarcado), sem instalar nada a mais

# 2c) ou lista via CLI
cargo run --release -- list --min-score 6

# 3) avisa no Telegram (só as que ainda não foram avisadas)
export TELEGRAM_TOKEN="..."
export TELEGRAM_CHAT_ID="..."
cargo run --release -- notify --min-score 6

# 4) controle do funil
cargo run --release -- mark 12 applied      # new | applied | interview | rejected | ignored
cargo run --release -- list --status applied --min-score 0
```

## Ajustes

- `config.toml`: termos de busca, localizações aceitas, pesos, palavras excluídas e feeds RSS.
- `jobs.db` (SQLite) guarda tudo; o token do Telegram nunca é gravado em arquivo.

## Estrutura

- `src/sources/`: uma fonte por arquivo; implemente o trait `Source` e registre em `sources::all()`.
- `src/score.rs`: pontuação por palavras (com limite de palavra, então "java" não casa com "javascript").
- `src/db.rs`: SQLite via `rusqlite`.
- `src/notify.rs`: Telegram via HTTP.
- `src/tui.rs`: interface de terminal com ratatui.

## Rodar todo dia (cron)

```cron
0 8 * * * cd ~/jobhunter && ./target/release/jobhunter fetch && ./target/release/jobhunter notify
```

## Próximos passos

1. Nova fonte de vagas (Gupy ou LinkedIn) implementando `Source`.
2. Nota extra por LLM (o provedor fica atrás de um trait, como `Source`).
3. Comando `export` gerando um CSV do funil.
4. Candidatura automática (trait `Applier` + Playwright para LinkedIn Easy Apply / Gupy).

## Docker

```bash
# Build da imagem
docker build -t jobhunter .

# Rodar a GUI em container com display (X11)
docker run -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix jobhunter gui

# Ou rodar o fetch/notify headless
docker run -e TELEGRAM_TOKEN=... -e TELEGRAM_CHAT_ID=... jobhunter fetch
docker run -e TELEGRAM_TOKEN=... -e TELEGRAM_CHAT_ID=... jobhunter notify

# Com docker-compose
docker-compose up jobhunter
```

## Instalador Windows (para FileZilla)

```bash
# Pré-requisitos: rustup target add x86_64-pc-windows-gnu && apt install mingw-w64 zip
./installer/windows/build-zip.sh
```

Gera `dist/jobhunter-windows.zip` contendo `jobhunter.exe` + `config.toml`.  
Coloque esse `.zip` na pasta compartilhada do FileZilla — o usuário baixa, descompacta e roda o `.exe`.
