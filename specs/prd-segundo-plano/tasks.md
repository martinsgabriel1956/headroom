# Tasks: Modo em segundo plano

Contrato: [techspec.md](techspec.md) · Produto: [prd.md](prd.md)

**Commits:**
- Branch `feat/segundo-plano`, criado a partir da `main`.
- Um commit por task, em Conventional Commits em português.

---

## 0. Branch e o que já estava pendente

- **Escopo:** abrir o branch e commitar o que já está feito: a logo e os documentos desta feature.
- **Contexto:** a logo (FG3) foi aplicada antes do SDD. Os arquivos estão modificados na `main`, sem commit.
- **Skills:** nenhuma.
- **Regras:** um assunto por commit.
- **Steps:**
  - `git switch -c feat/segundo-plano`.
  - Commit `feat: logo do Headroom`, com:
    - `app-icon.svg`;
    - `public/favicon.svg`;
    - `src-tauri/icons/*`;
    - a marca nova no `App.tsx`.
  - Commit `docs: PRD, techspec e tasks do modo em segundo plano`, com:
    - `specs/prd-segundo-plano/`;
    - `specs/README.md`;
    - `PRODUCT.md`.
- **Teste:** o `git status` fica limpo.

---

## 1. Dependências

- **Escopo:** feature `tray-icon` no Tauri, `tauri-plugin-single-instance` e `libc`.
- **Contexto:** a seção Arquivos relevantes do techspec.
  - O `libc` já vem como dependência transitiva, mas o `statvfs` e o `setpriority` precisam dele declarado.
  - O plugin de instância única vem do switch-terminal.
- **Skills:** `ponytail`.
- **Regras:** só as dependências citadas no techspec.
- **Steps:**
  - Em `src-tauri/Cargo.toml`:
    - `tauri = { version = "2.12.5", features = ["tray-icon"] }`;
    - `tauri-plugin-single-instance = "2"`;
    - `libc = "0.2"`.
  - Criar `mod background; mod notify;` como arquivos vazios.
- **Teste:** `cargo check` passa.

---

## 2. Lógica do monitor (`background.rs`, sem o laço)

- **Escopo:** constantes, `disk_full`, `Edge`, `scan_due` e `free_space`, com testes.
- **Contexto:** o techspec, em Interfaces principais e Estratégia de teste. É a lógica do PRD:
  - RF9: limite de 10% ou 20 GB;
  - RF16: avisar só quando a condição começa;
  - RF11 e RF12: a cada 6 h, só com a janela escondida.
- **Skills:** `ponytail`.
- **Regras:** funções puras separadas de qualquer chamada ao sistema.
- **Steps:**

```rust
pub const TICK: Duration = Duration::from_secs(60);
pub const DISK_CHECK: Duration = Duration::from_secs(15 * 60);
pub const SCAN_EVERY: Duration = Duration::from_secs(6 * 3600);
pub const RECOVERABLE_MIN: u64 = 5_000_000_000;
const FULL_BELOW: u64 = 20_000_000_000;

/// PRD RF9: menos de 10% livre ou menos de 20 GB, o que vier primeiro.
pub fn disk_full(free: u64, total: u64) -> bool {
    free.saturating_mul(10) < total || free < FULL_BELOW
}

/// PRD RF16: verdadeiro só na passagem de falso para verdadeiro.
#[derive(Default)]
pub struct Edge(bool);
impl Edge {
    pub fn rose(&mut self, now: bool) -> bool { let r = now && !self.0; self.0 = now; r }
}

pub fn scan_due(last: Option<Instant>, now: Instant, every: Duration, window_visible: bool) -> bool {
    !window_visible && last.is_none_or(|t| now.duration_since(t) >= every)
}

pub fn free_space(path: &Path) -> io::Result<(u64, u64)> // (livre, total) via libc::statvfs, f_bavail/f_blocks × f_frsize
```

- **Teste:**
  - `disk_full`:
    - 476 GB com 47 GB livres está cheio, e com 48 GB livres não está;
    - 100 GB com 19 GB livres está cheio, pela regra dos 20 GB;
    - 100 GB com 20 GB livres não está.
  - `Edge`: a sequência falso, verdadeiro, verdadeiro, falso, verdadeiro avisa só na 2ª e na 5ª.
  - `scan_due`: nunca com a janela visível; dispara sem `last`; antes do intervalo não dispara; no intervalo exato dispara.
  - `free_space` numa pasta temporária: `0 < livre <= total`.

---

## 3. Notificações (`notify.rs`)

- **Escopo:** enviar aviso pelo `notify-send` e abrir a janela no clique.
- **Contexto:** o techspec, em Pontos de integração e na decisão `notify-send`. Cobre o PRD RF10, RF13 e RF17.
- **Skills:** `ponytail`.
- **Regras:** o texto do aviso vem pronto de quem chama. Sem `notify-send`, o aviso só vai para o log.
- **Steps:**

```rust
static ICON: &[u8] = include_bytes!("../icons/128x128.png");

/// Grava o ícone uma vez em $XDG_RUNTIME_DIR/headroom/icon.png para o notify-send achar.
fn icon_path() -> Option<PathBuf>

pub fn notify(app: &AppHandle, summary: &str, body: &str) {
    let mut cmd = Command::new("notify-send");
    cmd.args(["--app-name=Headroom", "--action=default=Abrir"]);
    if let Some(icon) = icon_path() { cmd.arg(format!("--icon={}", icon.display())); }
    cmd.arg("--").arg(summary).arg(body).stdout(Stdio::piped());
    match cmd.spawn() {
        Err(e) => log::warn!("notify-send indisponível, aviso não exibido: {e}"),
        Ok(child) => {
            log::info!("aviso: {summary}");
            let app = app.clone();
            // --action espera o usuário; a thread fica parada até a notificação fechar.
            std::thread::spawn(move || {
                let clicked = child.wait_with_output().is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "default");
                if clicked { log::info!("aviso clicado"); crate::show_window(&app); }
            });
        }
    }
}
```

- **Teste:** `cargo check` passa. O comportamento é verificado na task 8, porque depende do servidor de notificações.

---

## 4. Resultado guardado e `snapshot`

- **Escopo:** `Scan.done` e o comando `snapshot`.
- **Contexto:** o techspec, em Modelos de dado e na decisão do `snapshot` sob demanda. Cobre o PRD RF14: a janela reaberta mostra a varredura automática sem varrer de novo.
- **Skills:** `ponytail`.
- **Regras:** a tela continua recebendo ids, nunca caminhos para apagar (v0, RF20).
- **Steps:**
  - `#[derive(Serialize, Clone)] struct DoneDto { millis, skipped, count, active: Vec<ActiveDto> }`.
  - `Scan` ganha `done: Option<DoneDto>`. O `run_scan` preenche esse campo antes de emitir o `Done`, e o evento passa a carregar o `DoneDto` (`ScanEvent::Done(DoneDto)` com `#[serde(flatten)]`, ou os mesmos campos, mantendo o JSON igual).
  - Código do `snapshot`:

```rust
#[derive(Serialize)] #[serde(rename_all = "camelCase")]
struct SnapshotDto { generation: u64, root: String, home: String, artifacts: Vec<ArtifactDto>, done: Option<DoneDto> }

#[tauri::command]
fn snapshot(state: State<AppState>) -> Option<SnapshotDto> // None antes da primeira varredura
```

  - Registrar `snapshot` no `invoke_handler`.
- **Teste:** depois de um `run_scan` numa árvore de exemplo, o `snapshot` devolve os mesmos ids dos eventos e `done.count` igual ao número de artefatos.

---

## 5. Bandeja e ciclo de vida da janela

- **Escopo:** bandeja com menu, X que esconde, instância única e aviso da primeira vez.
- **Contexto:** o PRD, RF1 a RF7, e a restrição de que sem bandeja o X fecha. No techspec, "Bandeja do Tauri, como no switch-terminal" e "Sem bandeja, sem segundo plano".
- **Skills:** `ponytail`.
- **Regras:** o id da bandeja é `headroom`, para não sobrescrever o ícone de outro app Tauri.
- **Steps:**

```rust
struct Tray(bool); // a bandeja está disponível nesta sessão?

fn tray_available() -> bool // busctl --user call org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus NameHasOwner s org.kde.StatusNotifierWatcher → "b true"

pub(crate) fn show_window(app: &AppHandle) // get_webview_window("main"): show, unminimize, set_focus

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let menu = Menu::with_items(app, &[
        &MenuItem::with_id(app, "open", "Abrir Headroom", true, None::<&str>)?,
        &MenuItem::with_id(app, "scan", "Escanear agora", true, None::<&str>)?,
        &PredefinedMenuItem::separator(app)?,
        &MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?,
    ])?;
    // id "headroom": no Linux vira o arquivo do ícone e o id do AppIndicator (ver switch-terminal).
    TrayIconBuilder::with_id("headroom").tooltip("Headroom").menu(&menu).show_menu_on_left_click(true)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "open" => show_window(app),
            "scan" => { show_window(app); let _ = app.emit("scan-now", ()); }
            "quit" => app.exit(0),
            _ => {}
        })
        /* .icon(default_window_icon) */ .build(app)?;
    Ok(())
}

/// PRD RF4: na primeira vez que a janela é fechada, explica que o app continua na bandeja.
fn close_notice_once(app: &AppHandle) // app_data_dir()/state.json { "closeNoticeShown": true }
```

  - No `Builder`:
    - `tauri_plugin_single_instance::init(|app, _, _| show_window(app))` entra como **primeiro** plugin.
    - `.setup`: calcula `tray_available()`, faz `manage(Tray(ok))` e, se `ok`, chama `setup_tray` e `background::spawn` (task 6). Registra no log se a bandeja está disponível.
    - `.on_window_event`: no `CloseRequested`, se `Tray(true)`, chama `api.prevent_close()`, depois `window.hide()` e `close_notice_once`.
  - Texto do aviso da primeira vez:
    - título: "O Headroom continua na bandeja";
    - corpo: "Ele avisa quando o disco encher ou quando valer limpar. Para sair, use Sair no menu da bandeja."
- **Teste:**
  - `cargo build` passa.
  - O comportamento é verificado na task 8: o X esconde, o aviso aparece uma vez, o menu funciona e uma segunda instância só traz a janela.

---

## 6. Monitor em segundo plano

- **Escopo:** o laço de `background::spawn`, a varredura automática em pool de baixa prioridade e os dois avisos.
- **Contexto:** o techspec, em Visão de componentes (`background.rs`) e nas decisões do tick de 60 s e do pool próprio. Cobre o PRD RF8 a RF13 e RF16. Em debug, `HEADROOM_FAST=1` encurta os intervalos e força as condições, para a verificação.
- **Skills:** `ponytail`.
- **Regras:** a varredura automática só lê. Nada aqui chama `run_delete` (PRD RF15).
- **Steps:**

```rust
pub fn spawn(app: AppHandle) {
    let fast = cfg!(debug_assertions) && std::env::var("HEADROOM_FAST").is_ok_and(|v| v == "1");
    let (tick, disk_every, scan_every) = if fast { (1s, 10s, 30s) } else { (TICK, DISK_CHECK, SCAN_EVERY) };
    std::thread::spawn(move || {
        // 2 threads em nice 10: a varredura automática não disputa CPU com o usuário (escolha do usuário).
        let pool = rayon::ThreadPoolBuilder::new().num_threads(2)
            .start_handler(|_| unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, 10); })
            .build().expect("pool de segundo plano");
        let (mut disk, mut recoverable) = (Edge::default(), Edge::default());
        let (mut last_disk, mut last_scan) = (None, Some(Instant::now())); // a varredura do início já conta
        loop {
            let now = Instant::now();
            let root = crate::current_root(&app);
            if last_disk.is_none_or(|t: Instant| now - t >= disk_every) { /* free_space → disk.rose(fast || disk_full) → notify */ }
            if scan_due(last_scan, now, scan_every, crate::window_visible(&app)) {
                last_scan = Some(now);
                if let Some(bytes) = pool.install(|| crate::background_scan(&app, root)) {
                    let _ = app.emit("scan-replaced", ());
                    if recoverable.rose(fast || bytes >= RECOVERABLE_MIN) { /* notify("16,4 GB para limpar", "em 176 projetos.") */ }
                }
            }
            std::thread::sleep(tick);
        }
    });
}
```

  - Em `lib.rs`:
    - `current_root(app)`: a raiz da última varredura, ou `$HOME`.
    - `window_visible(app)`.
    - `background_scan(app, root) -> Option<u64>`: roda o `run_scan` com um `emit` vazio. Devolve o `real_of` de todos os itens, ou `None` se a varredura foi cancelada, ou seja, se o `done` ficou vazio.
  - Formatação de GB para os textos: reaproveitar a regra do frontend (`6,8 GB` e `512 MB`) numa função `fmt_bytes` em Rust.
  - Textos:
    - disco cheio: título "Disco quase cheio", corpo "{livre} livres. O Headroom acha {recuperável} para limpar." O segundo trecho só entra se já houver varredura.
    - espaço recuperável: título "{recuperável} para limpar", corpo "em {n} projetos. Abra o Headroom para escolher o que apagar."
- **Teste:**
  - `fmt_bytes` tem teste unitário.
  - `background_scan` numa árvore de exemplo devolve `Some` com o real de todos os itens.
  - `cargo test` passa inteiro.

---

## 7. Frontend: varredura substituída e "Escanear agora"

- **Escopo:** `useHeadroom(onNewScan)`, escutando `scan-replaced` e `scan-now`.
- **Contexto:** o techspec, em Visão de componentes (Frontend). Cobre o PRD RF6 (Escanear agora) e RF14. A seleção é limpa num ponto só.
- **Skills:** `ponytail` e `impeccable`. O hook de design roda nos arquivos de UI.
- **Regras:** `useState` puro e o DESIGN.md. Sem mudança visual.
- **Steps:**
  - Em `scan.ts`:
    - Extrair a inclusão de um artefato do `onmessage` para uma função `add(a: Dto)`, usada também pelo `snapshot`.
    - `useHeadroom(onNewScan: () => void)` guarda o callback num ref. O `scan()` chama `onNewScan()` ao começar.
    - Ouvir com `listen` de `@tauri-apps/api/event`, só quando `live`:
      - `scan-replaced`: chama `invoke<Snapshot | null>('snapshot')`, depois `onNewScan()`, e substitui geração, raiz, projetos (via `add`), dias ativos e status.
      - `scan-now`: chama `scan(absRoot.current)`.
  - Em `App.tsx`:
    - `useHeadroom(clearSelection)`.
    - Os botões Escanear e de troca de raiz voltam a chamar `h.rescan` e `h.chooseRoot` direto, porque a limpeza acontece no `onNewScan`.
    - `chooseRoot` cancelado não varre e não limpa nada.
- **Teste:**
  - `npx tsc -b`, `npm test` e `npm run lint` passam, sem aviso novo.
  - O detector do impeccable não acusa nada.
  - No navegador, a tela de demonstração continua igual.

---

## 8. Verificação contra o PRD

- **Escopo:** rodar o app e conferir bandeja, avisos e ciclo de janela.
- **Contexto:** a Estratégia de teste do techspec. Esta parte é manual, porque o Wayland impede automatizar cliques.
- **Skills:** `run`.
- **Regras:** nada é apagado nesta verificação.
- **Steps:**
  - `HEADROOM_FAST=1 npm run tauri dev`.
  - Confirmar pelo log e pelas capturas:
    - a bandeja aparece;
    - o aviso de disco cheio sai uma vez e não se repete;
    - a varredura automática roda com a janela escondida, e o aviso de espaço recuperável sai;
    - o `scan-replaced` atualiza a tela ao reabrir.
  - Fechar a janela pelo `xdotool` (`windowclose` funciona em XWayland) e conferir no log que o app continua vivo e o aviso da primeira vez saiu.
  - Rodar o app uma segunda vez e conferir que nenhuma janela nova abre.
  - **Para você:** clicar no ícone da bandeja e nos avisos, e conferir Abrir, Escanear agora e Sair.
- **Teste:** os resultados ficam no resumo final. Não gera commit, a não ser que precise de correção.
