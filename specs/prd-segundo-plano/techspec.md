# Techspec: Modo em segundo plano

## Resumo executivo

O segundo plano mora no backend Rust e reaproveita o que a v0 já decidiu: o estado da varredura no backend com geração, o `run_scan` e o `real_of`. Ele acrescenta três peças.

- **Bandeja e ciclo de vida.** Usam o `tray-icon` do próprio Tauri e o `tauri-plugin-single-instance`, no mesmo padrão já provado no switch-terminal do usuário e no mesmo desktop (COSMIC). O X esconde a janela; o menu da bandeja tem Abrir, Escanear agora e Sair.
- **Um monitor.** É uma thread que acorda a cada 60 s. A cada 15 min ela confere o espaço livre com `statvfs`. A cada 6 h, se a janela estiver escondida, ela roda o mesmo `run_scan` num pool `rayon` de 2 threads com prioridade reduzida.
- **Os avisos.** Usam `notify-send` com uma ação padrão. O clique devolve "default" no stdout, e a thread que esperava mostra a janela.

A interface não fica sabendo de nada disso até precisar. Quando uma varredura automática termina, o backend emite o evento global `scan-replaced`. O frontend responde chamando um comando novo, `snapshot`, que devolve o estado atual inteiro. O "Escanear agora" da bandeja emite `scan-now`, e o frontend reusa o próprio `rescan`. Assim o caminho manual continua sendo o único que limpa a seleção.

## Arquitetura

### Visão de componentes

**Backend (`src-tauri/src/`)**

- `background.rs` (novo):
  - **Lógica pura e testável:**
    - `disk_full(free, total)` aplica o "10% ou 20 GB" do RF9.
    - `Edge` é a borda de subida do RF16: só avisa quando a condição passa de falsa para verdadeira.
    - `scan_due(última, agora, intervalo, janela_visível)` decide se a varredura automática deve rodar (RF11 e RF12).
  - **Partes de sistema:**
    - `free_space(path)` usa `libc::statvfs`, com `f_bavail` e `f_blocks` multiplicados por `f_frsize`.
    - `spawn(app)` é a thread do monitor.
  - **A varredura automática:**
    - Chama o `run_scan` existente dentro de `pool.install`. O pool `rayon` é dedicado, com 2 threads e `setpriority(nice 10)` no `start_handler`. O `rayon::scope` do `scan.rs` herda esse pool.
    - Ao terminar, emite `scan-replaced`.
    - Avalia a `Edge` de espaço recuperável com o `real_of` de todos os itens.
- `notify.rs` (novo): `notify(app, título, corpo)`.
  - Roda `notify-send --app-name=Headroom --icon=<png> -A default=Abrir -- título corpo` numa thread, lê o stdout e chama `show_window` se vier `default`.
  - O ícone é o `icons/128x128.png`, embutido com `include_bytes!` e gravado uma vez em `$XDG_RUNTIME_DIR/headroom/icon.png`.
  - Sem `notify-send`, registra um aviso no log e segue, sem erro para o usuário.
- `lib.rs` (modificado):
  - **Bandeja:**
    - `setup_tray`: id `headroom`, para não colidir com outro app Tauri; ícone da janela; menu Abrir Headroom, Escanear agora, separador e Sair.
  - **Janela:**
    - `show_window`: `show`, `unminimize`, `set_focus`.
    - `on_window_event`: no `CloseRequested`, `prevent_close` e `hide`, mas só se a bandeja estiver disponível. Também dispara o aviso de primeira vez do RF4.
  - **Instância única:** o `single_instance` chama `show_window`.
  - **Varredura:**
    - O comando novo `snapshot`.
    - `Scan` passa a guardar o `Done` da última varredura (tempo, pulados, ativos), para o `snapshot` devolvê-lo.
- **Detecção da bandeja:** `busctl --user call … NameHasOwner s org.kde.StatusNotifierWatcher` uma vez no `setup`. Se o comando não existir ou responder `false`, a bandeja é tratada como indisponível, o X fecha o app de verdade e o monitor não é iniciado. Isso cumpre a restrição do PRD.

**Frontend (`src/`)**

- `scan.ts`:
  - **Novos eventos:** `useHeadroom(onNewScan)` recebe um callback, chamado sempre que começa uma varredura nova: manual, da bandeja ou substituída em segundo plano. Também escuta `scan-replaced`, que chama `snapshot` e reconstrói projetos e status, e `scan-now`, que chama `rescan`.
  - **Uso existente:** o `App.tsx` passa `clearSelection` como callback e deixa de limpar a seleção por conta própria nos handlers.

**Fluxo**

```
monitor (60 s) ─┬─ 15 min: statvfs → disk_full? → Edge ↑ → notify("Disco quase cheio…")
                └─ 6 h e janela escondida: pool baixo → run_scan → emit scan-replaced
                                                            → real_of ≥ 5 GB? → Edge ↑ → notify("… para limpar")
janela aberta ← clique na notificação / Abrir / outra instância
frontend: scan-replaced → snapshot() → estado    ·   scan-now → rescan()
```

## Design de implementação

### Interfaces principais

```rust
// background.rs
pub const DISK_CHECK: Duration = Duration::from_secs(15 * 60);
pub const SCAN_EVERY: Duration = Duration::from_secs(6 * 3600);
pub const RECOVERABLE_MIN: u64 = 5_000_000_000;
pub fn disk_full(free: u64, total: u64) -> bool;              // free < total/10 || free < 20 GB
#[derive(Default)] pub struct Edge(bool);
impl Edge { pub fn rose(&mut self, now: bool) -> bool }        // true só na passagem falso → verdadeiro
pub fn scan_due(last: Option<Instant>, now: Instant, every: Duration, window_visible: bool) -> bool;
pub fn free_space(path: &Path) -> io::Result<(u64 /*livre*/, u64 /*total*/)>;
pub fn spawn(app: AppHandle);

// notify.rs
pub fn notify(app: &AppHandle, summary: &str, body: &str);

// lib.rs
#[tauri::command] fn snapshot(state: State<AppState>) -> Option<SnapshotDto>;
fn show_window(app: &AppHandle);
fn tray_available() -> bool;
```

```ts
export function useHeadroom(onNewScan: () => void): { /* o mesmo de hoje */ }
```

### Modelos de dado

```ts
type SnapshotDto = { generation: number; root: string; home: string; artifacts: ArtifactDto[]
                     done: { millis: number; skipped: number; count: number; active: ActiveDto[] } | null }
```

- **Estado novo no backend:**
  - `Scan.done: Option<DoneSummary>`.
  - Um `Monitor` com `disk: Edge`, `recoverable: Edge` e `last_scan: Option<Instant>`. Ele vive só em memória: depois de reiniciar o app, uma condição que ainda vale avisa de novo, e isso é aceito.
- **Persistência:** `app_data_dir()/state.json` com `{ "closeNoticeShown": true }`. É o único dado salvo em disco pela feature.

### Endpoints de API

Sem HTTP. A superfície nova é o comando `snapshot` e os eventos globais `scan-replaced` (com `generation`) e `scan-now`.

## Pontos de integração

- **Bandeja (AppIndicator / StatusNotifierItem):** em tempo de execução precisa de `libayatana-appindicator3`, que já está instalada. No Linux, o clique na bandeja sempre abre o menu, e o AppIndicator não entrega o evento de clique ao app. Por isso o RF5 é cumprido pelo primeiro item do menu, "Abrir Headroom".
- **Notificações:** `notify-send` (libnotify-bin). Uma ação `default` é disparada ao clicar no corpo da notificação na maioria dos servidores, incluindo o do COSMIC. Se o servidor não suportar ações, o aviso aparece sem clique e mais nada quebra. Como o `-A` espera a resposta, cada notificação ocupa uma thread até ser fechada ou expirar. São poucas, então isso é aceito.
- **`statvfs`:** um erro, como a raiz removida, é registrado no log e ignorado até a próxima checagem.

## Estratégia de teste

**Unitário (Rust, `cargo test`):**
- `disk_full` nas bordas: 10% exato, 20 GB exato, disco pequeno e disco grande.
- `Edge`: avisa só na subida; continuar verdadeiro não avisa; descer e subir avisa de novo.
- `scan_due`: nunca com a janela visível; na primeira vez sem `last`; só depois do intervalo.
- `free_space` numa pasta temporária: total > 0 e livre ≤ total.

**Integração (Rust):**
- `run_scan` seguido de `snapshot` devolve os mesmos ids e o `done` preenchido.
- O pool de baixa prioridade roda o `run_scan` de uma árvore de exemplo e chega ao `Done`.

**Manual** (bandeja, notificações e ciclo de janela não têm automação neste ambiente Wayland). Em builds de debug, `HEADROOM_FAST=1` encurta os intervalos (disco a cada 10 s, varredura a cada 30 s) e força as duas condições a valer. Isso permite ver os dois avisos, o clique abrindo a janela e a regra de não repetir. Conferências:
- o X esconde a janela;
- o aviso da primeira vez sai só uma vez;
- Sair encerra o app;
- uma segunda instância só traz a janela.

**Frontend:** `npx tsc -b` e verificação manual de que `scan-replaced` atualiza a planta ao reabrir a janela.

## Sequenciamento

1. **Dependências e configuração:** feature `tray-icon` no `tauri` e `tauri-plugin-single-instance`.
2. **`background.rs`:** só a lógica pura e o `free_space`, com testes. Não depende de nada novo.
3. **`notify.rs`.**
4. **`lib.rs`:** bandeja, fechar que esconde, instância única, detecção da bandeja, `Scan.done` e `snapshot`. Depois `background::spawn` com o monitor e o pool de baixa prioridade.
5. **Frontend:** `onNewScan`, `scan-replaced` e `scan-now`.
6. **Verificação manual** com `HEADROOM_FAST`.

## Observabilidade

O `tauri-plugin-log` já existente registra:
- se a bandeja está disponível;
- cada checagem de disco que muda de estado (livre e total);
- início e fim da varredura automática, reaproveitando a linha de `run_scan`;
- cada notificação enviada e cada clique;
- a ausência do `notify-send`.

## Considerações técnicas

### Decisões principais

- **Bandeja do Tauri (`tray-icon`) e `single-instance`, como no switch-terminal.** O padrão já funciona no mesmo desktop do usuário. Isso inclui o id próprio da bandeja, para não sobrescrever o ícone de outro app Tauri. Escrever o protocolo StatusNotifierItem à mão foi descartado.
- **`notify-send` em vez de `tauri-plugin-notification` ou `notify-rust`.** É a escolha do usuário, alinhada ao switch-terminal. O plugin do Tauri não entrega o clique da notificação no desktop, e o RF17 precisa disso. O `-A default=` resolve o clique sem dependência nova.
- **Uma thread de monitor com tick de 60 s, e não timers separados.** Os dois relógios ficam num lugar só e a lógica de "está na hora" é uma função pura testável. O custo ocioso é acordar uma vez por minuto.
- **Varredura automática num pool `rayon` próprio, com 2 threads e nice 10.** É a escolha do usuário. O `rayon::scope` do `scan.rs` roda no pool de quem chama, então não foi preciso mudar o `scan.rs`. Usar o pool global foi descartado porque ocupa todos os núcleos.
- **Atualização da tela por `snapshot` sob demanda, e não por streaming.** Ninguém está olhando enquanto a varredura automática roda. Transmitir cada artefato para uma janela escondida é desperdício, e o `snapshot` também serve para qualquer reabertura futura.
- **Estado das bordas só em memória.** É o mais simples. Avisar de novo depois de reiniciar uma condição que ainda vale é aceitável. Persistir esse estado foi descartado por não valer o arquivo a mais.
- **Sem bandeja, sem segundo plano.** Sem a detecção, o X esconderia a janela num desktop sem bandeja e deixaria um processo impossível de encerrar pela interface.

### Riscos conhecidos

- **Ação de clique depende do servidor de notificações.** Se o COSMIC ignorar o `default`, o RF17 fica sem efeito ali. Isso vai ser conferido na verificação manual. A alternativa seria uma ação visível "Abrir".
- **O RF5 vira "Abrir pelo menu" no Linux,** por limitação do AppIndicator. O PRD precisa registrar isso.
- **`busctl` ausente** (sistemas sem systemd): a bandeja seria tratada como indisponível mesmo existindo. É aceito, porque é o lado seguro.
- **Varredura automática e manual juntas:** a geração cancela a anterior. Se o usuário abrir a janela e escanear durante uma varredura automática, a automática é descartada.
- **Corrida entre `scan-replaced` e a janela aberta:** a varredura automática só começa com a janela escondida. Se a janela abrir no meio, o `scan-replaced` chega depois e substitui os dados. Isso pode acontecer enquanto o usuário seleciona, e aí a seleção é limpa pelo `onNewScan`. É raro e é aceito; fica registrado aqui.

### Conformidade com regras do projeto

- **Projeto sem `CLAUDE.md`.** Seguem PRODUCT.md, DESIGN.md e o techspec da v0.
- **Estado e exclusão:** o estado continua no backend com ids e geração, e a exclusão segura (RF20 a RF23 da v0) continua intocada.
- **Visual:** a única peça é o ícone da bandeja, que é a logo. As notificações seguem a voz sem alarme do PRODUCT.md.
- **Desvios:** nenhum.

### Conformidade com skills disponíveis

- **`ponytail`:** sem dependência nova além das duas citadas, e um teste para cada lógica não trivial.
- **`sdd-tasks` e `sdd-review`:** continuam o pipeline.
- **`run`:** para a verificação manual com o app rodando.

### Arquivos relevantes

- **Backend:**
  - `src-tauri/Cargo.toml`, com a feature `tray-icon` e o `tauri-plugin-single-instance`.
  - `src-tauri/src/background.rs` e `notify.rs` (novos).
  - `src-tauri/src/lib.rs`.
  - O ícone vem de `src-tauri/icons/128x128.png`.
- **Frontend:** `src/scan.ts` e `src/App.tsx`.

## Histórico

**2026-11-09**. Documento criado. O usuário escolheu `notify-send` e a varredura automática em baixa prioridade.
