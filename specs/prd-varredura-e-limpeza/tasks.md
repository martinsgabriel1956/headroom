# Tasks: Varredura e limpeza reais

Contrato: [techspec.md](techspec.md) · Produto: [prd.md](prd.md)

**Commits.** O repositório ainda não tem commits.
- **Task 0:** commit de base em `main` com tudo que já existe. É a tela da Planta Baixa, os specs, os documentos e o scaffold do Tauri.
- **Tasks 1 a 8:** rodam no branch `feat/varredura-e-limpeza`, com um commit por task.
- **Convenção:** a dos outros projetos do usuário, Conventional Commits em português e curto. Por exemplo, `feat: regras de detecção de artefatos`.

---

## 0. Commit de base

- **Escopo:** registrar o estado atual antes da feature.
- **Contexto:** sem um commit de base, o diff de cada task misturaria a tela, os specs e o scaffold.
- **Skills:** nenhuma.
- **Regras:** o `.impeccable/` entra só com o que é para versionar: `design.json` e `surfaces/`. As pastas `mocks/`, `questions/` e `review/` já se ignoram, e o `hook.cache.json` já está no exclude local.
- **Steps:**
  - `git add -A` e `git commit -m "chore: base do projeto"` em `main`.
  - Depois, `git switch -c feat/varredura-e-limpeza`.
- **Teste:** o `git status` fica limpo.

---

## 1. Configurar o app Tauri

- **Escopo:** nome, janela, dependências, permissão do diálogo e esqueleto do `lib.rs`.
- **Contexto:** o scaffold veio com nomes genéricos (`app`, `com.tauri.dev`, janela de 800×600). O techspec (Sequenciamento, passo 5) pede o app Headroom em 1280×800, com mínimo de 1024×680, o diálogo nativo e o `rayon`.
- **Skills:** `ponytail`.
- **Regras:** só dependências citadas no techspec.
- **Steps:**
  - `src-tauri/Cargo.toml`: `name = "headroom"`, `description = "Limpa artefatos regeneráveis de projetos"`, `authors = ["Gabriel Martins"]`. Na seção lib, `name = "headroom_lib"`. Dependências adicionadas: `rayon = "1"` e `tauri-plugin-dialog = "2"`.
  - `src-tauri/src/main.rs`: chama `headroom_lib::run()`.
  - `src-tauri/tauri.conf.json`: `productName: "Headroom"` e `identifier: "dev.headroom.app"`. Na janela, `title: "Headroom"`, `width: 1280`, `height: 800`, `minWidth: 1024` e `minHeight: 680`.
  - `src-tauri/capabilities/default.json`: `"permissions": ["core:default", "dialog:allow-open"]`.
  - `lib.rs`: registra `tauri_plugin_dialog::init()`. O log deixa de ser só de debug e passa a valer sempre, no nível Info, para servir à observabilidade.
  - Módulos vazios: `mod rules; mod measure; mod scan; mod delete;`.
- **Teste:** `cargo check` em `src-tauri` passa e `npm run tauri dev` abre a janela com a tela atual.

---

## 2. Regras de detecção (`rules.rs`)

- **Escopo:** a tabela do RF10 como função pura.
- **Contexto:** o techspec (Arquitetura, `rules.rs`) usa a mesma função na descoberta e na revalidação. A evidência mostrada na tela (RF12) nomeia o arquivo que foi achado de fato.
- **Skills:** `ponytail`.
- **Regras:** nenhuma específica.
- **Steps:**

```rust
use std::path::Path;

enum Need { Beside(&'static [&'static str]), Inside(&'static str), Always }

const RULES: &[(&str, Need)] = &[
    ("node_modules", Need::Beside(&["package.json"])),
    ("target", Need::Beside(&["Cargo.toml"])),
    (".venv", Need::Inside("pyvenv.cfg")),
    ("venv", Need::Inside("pyvenv.cfg")),
    ("__pycache__", Need::Always),
    (".pytest_cache", Need::Always),
    (".next", Need::Beside(&["package.json"])),
    (".turbo", Need::Beside(&["package.json"])),
    (".gradle", Need::Beside(&["build.gradle", "build.gradle.kts", "settings.gradle"])),
];

pub struct Rule { pub kind: &'static str, pub evidence: String }

/// `dir` é a pasta candidata. Devolve a regra que a aprova, olhando o disco agora.
pub fn match_rule(dir: &Path) -> Option<Rule> {
    let name = dir.file_name()?.to_str()?;
    let (kind, need) = RULES.iter().find(|(k, _)| *k == name)?;
    let evidence = match need {
        Need::Beside(files) => files.iter().find(|f| dir.parent()?.join(f).is_file()).map(|f| format!("{f} ao lado"))?,
        Need::Inside(f) => dir.join(f).is_file().then(|| format!("{f} dentro"))?,
        Need::Always => "sempre cache".into(),
    };
    Some(Rule { kind, evidence })
}
```

  O `?` dentro do `find` não compila numa closure que devolve `bool`. Na implementação, o `parent` é resolvido antes, fora da closure. A forma acima mostra só a decisão.

- **Teste** (`#[cfg(test)]` no próprio arquivo, árvore real em `temp_dir`):
  - Cada linha da tabela casa com a evidência certa e não casa sem ela.
  - `dist` e `build` nunca casam.
  - `.gradle` mostra `settings.gradle ao lado` quando só esse arquivo existe.

---

## 3. Medição (`measure.rs`)

- **Escopo:** tamanho aparente, tamanho real com hardlinks e modificação mais recente de uma pasta. Também a combinação de várias pastas (`real_of`).
- **Contexto:** este é o diferencial do produto (PRD, RF16 a RF19). Segue o techspec em Modelos de dado e Decisões principais: o real é calculado por blocos e ligações. Um arquivo só conta quando todas as ligações dele estão no conjunto.
- **Skills:** `ponytail`.
- **Regras:** nenhuma específica.
- **Steps:**

```rust
use rayon::prelude::*;
use std::{collections::HashMap, fs, os::unix::fs::MetadataExt, path::Path};

pub type Inode = (u64, u64);

#[derive(Default, Clone)]
pub struct Measured {
    pub apparent: u64,
    pub single: u64,                              // blocos de diretórios e de arquivos com 1 ligação
    pub newest: i64,                              // mtime mais recente, em segundos unix
    pub linked: HashMap<Inode, (u64, u64, u64)>,  // (ligações aqui dentro, nlink, bytes)
}

impl Measured {
    fn add(&mut self, md: &fs::Metadata) {
        let bytes = md.blocks() * 512;
        self.newest = self.newest.max(md.mtime());
        if !md.is_dir() { self.apparent += md.len() }
        if md.is_dir() || md.nlink() == 1 { self.single += bytes }
        else { self.linked.entry((md.dev(), md.ino())).or_insert((0, md.nlink(), bytes)).0 += 1 }
    }
    fn merge(mut self, o: Measured) -> Measured { /* soma apparent e single, max de newest, soma as contagens por inode */ }
    pub fn real(&self) -> u64 { real_of([self]) }
}

pub fn real_of<'a>(set: impl IntoIterator<Item = &'a Measured>) -> u64 {
    let mut single = 0;
    let mut links: HashMap<Inode, (u64, u64, u64)> = HashMap::new();
    for m in set {
        single += m.single;
        for (k, (c, n, b)) in &m.linked { links.entry(*k).or_insert((0, *n, *b)).0 += c }
    }
    single + links.values().filter(|(c, n, _)| c >= n).map(|(_, _, b)| b).sum::<u64>()
}

/// Mede `dir` sem seguir links e sem trocar de disco (`dev`). Para entre diretórios se `cancel()`.
pub fn measure(dir: &Path, dev: u64, cancel: &(dyn Fn() -> bool + Sync)) -> Measured {
    let mut m = Measured::default();
    if let Ok(md) = fs::symlink_metadata(dir) { m.add(&md) }
    walk(dir, dev, cancel).into_iter().fold(m, Measured::merge) // walk paraleliza os filhos com par_iter + reduce
}
```

- **Teste:**
  - O aparente é a soma dos tamanhos dos arquivos.
  - Um arquivo com hardlink para fora da pasta não conta no real.
  - Dois hardlinks dentro da mesma pasta contam uma vez.
  - O `real_of` de duas pastas que dividem um inode conta esse inode uma vez, e cada pasta sozinha não o conta.
  - Um symlink para um arquivo grande não soma o tamanho do alvo.

---

## 4. Varredura (`scan.rs`)

- **Escopo:** o percurso paralelo da raiz.
- **Contexto:** o techspec, em Arquitetura (`scan.rs`) e Decisões principais, define o que o percurso faz:
  - Usa `rayon::scope` e não segue links.
  - Corta em `st_dev` diferente e em `.git`.
  - Não desce em artefato e mede cada um em paralelo.
  - Guarda os diretórios com arquivo recente.
  - Acha o projeto pelo `.git` mais próximo.
  - Cobre RF4 e RF6 a RF8, RF13 e RF14.
- **Skills:** `ponytail`.
- **Regras:** nenhuma específica.
- **Steps:**

```rust
pub struct Found { pub id: u32, pub path: PathBuf, pub project: PathBuf, pub kind: &'static str,
                   pub rule: String, pub measured: Measured }

#[derive(Default)]
pub struct Counters { pub dirs: AtomicU64, pub skipped: AtomicU64 }

/// Percorre `root`, que já vem canonicalizado. Chama `found` para cada artefato já medido, de várias threads.
/// Devolve os diretórios fora de artefatos com arquivo modificado depois de `cutoff` (unix s) e o mtime mais novo de cada um.
pub fn scan(root: &Path, cutoff: i64, counters: &Counters,
            cancel: &(dyn Fn() -> bool + Sync), found: &(dyn Fn(Found) + Sync)) -> Vec<(PathBuf, i64)>

fn visit<'s>(s: &rayon::Scope<'s>, ctx: &'s Ctx<'s>, dir: PathBuf) {
    if (ctx.cancel)() { return }
    let Ok(entries) = fs::read_dir(&dir) else { ctx.counters.skipped.fetch_add(1, Relaxed); return };
    ctx.counters.dirs.fetch_add(1, Relaxed);
    let mut newest = 0;
    for e in entries.flatten() {
        let Ok(md) = e.metadata() else { continue };     // DirEntry::metadata não segue symlink
        let path = e.path();
        if md.is_file() { newest = newest.max(md.mtime()); continue }
        if !md.is_dir() || md.dev() != ctx.dev || e.file_name() == ".git" { continue }
        match rules::match_rule(&path) {
            Some(rule) => s.spawn(move |_| ctx.emit(path, rule)),   // mede e chama found
            None => s.spawn(move |s| visit(s, ctx, path)),
        }
    }
    if newest >= ctx.cutoff { ctx.recent.lock().unwrap().push((dir, newest)) }
}

/// Pasta git mais próxima entre o artefato e a raiz. Sem git, o pai do artefato.
pub fn project_of(artifact: &Path, root: &Path) -> PathBuf
```

  Os ids vêm de um `AtomicU32` no `Ctx`. Mais uma função pura, `active_projects(recent, projects) -> Vec<(PathBuf, i64)>`, devolve para cada projeto o mtime mais novo entre os diretórios recentes que começam com o caminho dele.

- **Teste:**
  - Um `node_modules` aninhado dentro de outro aprovado não aparece.
  - Um symlink para uma pasta com `node_modules` não é seguido.
  - Um `.venv` dentro de uma pasta oculta aparece.
  - Nada dentro de `.git` é visitado.
  - O projeto de `a/src-tauri/target`, com `a/.git`, é `a`. Sem `.git`, é o pai.
  - Um arquivo recente fora do artefato deixa o projeto ativo. Um arquivo recente só dentro do artefato não deixa.
  - Uma pasta com `chmod 000` conta em `skipped` e a varredura continua.
  - `#[ignore]`: o tempo de `~/Documentos/development`, impresso para conferência manual.

---

## 5. Exclusão (`delete.rs`)

- **Escopo:** revalidar e apagar um artefato.
- **Contexto:** a fronteira de confiança do PRD (RF20 a RF23). O techspec (Arquitetura, `delete.rs`) define as verificações e o tratamento de exclusão parcial.
- **Skills:** `ponytail`.
- **Regras:** nunca apagar sem revalidar. Toda recusa tem um motivo legível em português.
- **Steps:**

```rust
/// `root` canonicalizado na varredura. Confere o disco agora, não o que a varredura viu.
pub fn revalidate(root: &Path, dev: u64, f: &Found) -> Result<(), String> {
    let md = fs::symlink_metadata(&f.path).map_err(|_| "a pasta não existe mais")?;
    if md.file_type().is_symlink() { return Err("virou link simbólico".into()) }
    if !md.is_dir() { return Err("não é mais uma pasta".into()) }
    if md.dev() != dev { return Err("está em outro disco".into()) }
    let parent = f.path.parent().and_then(|p| fs::canonicalize(p).ok()).ok_or("caminho inválido")?;
    if !parent.starts_with(root) { return Err("saiu da pasta varrida".into()) }
    match rules::match_rule(&f.path) {
        Some(r) if r.kind == f.kind => Ok(()),
        _ => Err("a regra não vale mais".into()),
    }
}

pub fn remove(f: &Found) -> Result<(), String> {
    fs::remove_dir_all(&f.path).map_err(|e| match f.path.exists() {
        true => format!("apagado em parte: {e}"),
        false => e.to_string(),
    })
}
```

- **Teste:**
  - A exclusão recusa quando o `package.json` sumiu depois da varredura.
  - Recusa quando a pasta foi trocada por um symlink.
  - Recusa quando o caminho aponta para fora da raiz.
  - Recusa quando a pasta não existe mais.
  - Quando tudo confere, a pasta some.
  - O `remove_dir_all` não apaga o alvo de um symlink que estava dentro do artefato.

---

## 6. Comandos, estado e eventos (`lib.rs`)

- **Escopo:** os comandos `scan`, `measure` e `delete`, o estado com geração e o streaming por `Channel`.
- **Contexto:** o techspec cobre isto em três seções:
  - Interfaces principais e Modelos de dado: os eventos e DTOs com `serde(tag = "type")` em camelCase.
  - Fluxo de dados: a geração cancela a varredura anterior, e o progresso sai a cada 100 ms.
  - Pontos de integração: o `$HOME`.
- **Skills:** `ponytail`.
- **Regras:** os comandos recebem ids, nunca caminhos para apagar (PRD, Restrições).
- **Steps:**

```rust
struct Scan { generation: u64, root: PathBuf, dev: u64, items: HashMap<u32, Found> }
#[derive(Default)] struct AppState { generation: AtomicU64, last: Mutex<Option<Scan>> }

#[derive(Serialize, Clone)] #[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
enum ScanEvent { Started { generation: u64, root: String, home: String }, Artifact { artifact: ArtifactDto },
                 Progress { dirs: u64 }, Done { millis: u64, skipped: u64, count: u32, active: Vec<ActiveDto> } }

/// Núcleo testável, sem Tauri: tudo que o comando faz, emitindo por `emit`.
fn run_scan(state: &AppState, root: PathBuf, emit: &(dyn Fn(ScanEvent) + Sync)) -> Result<(), String>

#[tauri::command]
async fn scan(app: AppHandle, root: Option<String>, on_event: Channel<ScanEvent>) -> Result<(), String> {
    let root = root.map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(PathBuf::from)).ok_or("Sem $HOME")?;
    tauri::async_runtime::spawn_blocking(move || run_scan(&app.state::<AppState>(), root, &|e| { let _ = on_event.send(e); }))
        .await.map_err(|e| e.to_string())?
}
```

  O `run_scan` faz o seguinte:
  - Canonicaliza a raiz e lê o `st_dev` dela.
  - Incrementa a geração e grava um `Scan` vazio.
  - Emite `Started`.
  - Roda `scan::scan` numa thread, com `cancel = generation != atual`. Uma segunda thread emite `Progress` a cada 100 ms até a varredura acabar.
  - No `found`, insere o item no estado se a geração ainda bater e emite `Artifact`.
  - No fim, calcula `active_projects` e emite `Done`. Também registra no log a raiz, o tempo, as pastas, os artefatos e os pulados.

  `measure(generation, ids)` devolve o `real_of` dos itens da geração atual. Se a geração for antiga, devolve um erro.

  `delete(generation, ids, on_event)` funciona assim:
  - Roda em `spawn_blocking` e segura o lock do estado do começo ao fim. Isso também impede duas exclusões ao mesmo tempo (RF24).
  - Para cada id, emite `Deleting`, depois roda `revalidate` e `remove`, e emite `Deleted` ou `Refused { reason }`.
  - O `freed` é o `real_of` dos itens apagados, calculado antes de tirá-los do estado.
  - Tira do estado os apagados e os apagados em parte.
  - Registra cada resultado no log e devolve `DeleteReport`.

- **Teste:**
  - `run_scan` sobre uma árvore de exemplo, com um coletor de eventos, emite `Started`, depois um ou mais `Artifact` e por último `Done`, nessa ordem.
  - Os ids batem com o estado.
  - Uma segunda varredura invalida a geração antiga: `measure` com a geração velha devolve erro.
  - `cargo test` passa inteiro.

---

## 7. Frontend: hook `useHeadroom` (`src/scan.ts`) e tipos

- **Escopo:** ligar a interface aos comandos. Fora do Tauri, a tela cai nos dados de exemplo.
- **Contexto:** o techspec (Arquitetura, Frontend) pede para encapsular `invoke`, `Channel` e o diálogo num hook. A tela fica sem saber de onde vêm os dados.
- **Skills:** `ponytail`.
- **Regras:** `useState` puro, sem biblioteca de estado (PRODUCT.md, Stack).
- **Steps:**
  - Em `data.ts`, `Artifact` ganha `id: string`, com o id numérico do backend convertido para string. `Project.days` vira `number`, valendo `Infinity` quando o projeto não está ativo ou ainda não se sabe.
  - Em `scan.ts`:

```ts
export type Status = { phase: 'scanning'; dirs: number } | { phase: 'done'; millis: number; skipped: number; count: number }
  | { phase: 'error'; message: string }
export type DeleteResult = { freed: number; refused: { label: string; reason: string }[] }

export function useHeadroom(): {
  live: boolean; root: string; projects: Project[]; status: Status
  chooseRoot(): Promise<void>; rescan(): void
  measure(ids: string[]): Promise<number>
  remove(ids: string[], onProgress: (done: number) => void): Promise<DeleteResult>
}
```

  - **Varredura:** começa ao montar. Cada `artifact` entra num `Map` por projeto. Para não renderizar a cada evento, o estado é atualizado no máximo uma vez por quadro, com `requestAnimationFrame`. O `done` aplica os `days` dos projetos ativos.
  - **`chooseRoot`:** chama `open({ directory: true, defaultPath: root })`. Se voltar `null`, não faz nada. Se voltar uma pasta, varre de novo.
  - **Exibição dos caminhos:** trocam o `home` do `Started` por `~`.
  - **Fora do Tauri:**
    - `live` é `false` e a lista vem de `SAMPLE`.
    - `measure` soma os `real` localmente.
    - `remove` tira os itens do estado local.
- **Teste:** `npx tsc -b` passa e `npm run dev` no navegador continua mostrando os dados de exemplo.

---

## 8. Frontend: `App.tsx` com dados reais

- **Escopo:** trocar `SAMPLE` pelo hook e adicionar os estados de varredura, exclusão e falha.
- **Contexto:** a seção Experiência do usuário do PRD lista o que muda na tela. A estrutura e o DESIGN.md continuam valendo: andamento na voz de anotação, falhas em ink e graphite, nunca em brick.
- **Skills:** `ponytail` e `impeccable`. Se o hook de design apontar algo nos arquivos de UI, trato no fluxo dele.
- **Regras (DESIGN.md):**
  - One Blue: o azul só marca seleção.
  - Whole Figure: números de GB nunca aparecem cortados.
  - Last Door: o brick só aparece no botão final de apagar.
- **Steps:**
  - `const h = useHeadroom()`. Os `projects` vêm de `h.projects`.
  - A seleção é zerada quando começa uma nova varredura.
  - **Total selecionado:** com debounce de 150 ms, `h.measure(ids)` alimenta `pickedReal`. Esse valor é o do botão Limpar, da confirmação e da linha de cota. Enquanto a medição não volta, a tela mostra a soma local.
  - **Cabeçalho da planta:** fica numa região `aria-live="polite"`. Mostra "Varrendo… 12.408 pastas lidas" durante a varredura. No fim, mostra "{n} artefatos em {s} s", e acrescenta "· {k} pastas sem permissão" quando `k > 0`.
  - **Pasta raiz:** o botão mostra `h.root` com `~` e chama `h.chooseRoot()`. O botão Escanear chama `h.rescan()`.
  - **Selo "Dados de exemplo":** só aparece quando `!h.live`.
  - **Estados da planta:**
    - Erro: a planta mostra a mensagem no lugar.
    - Varredura sem nada achado ainda: "Procurando artefatos…".
    - Varredura terminada sem nada: a mensagem vazia que já existe.
  - **Confirmação:**
    - "Apagar" chama `h.remove`. Durante a exclusão, os botões ficam desabilitados e o principal mostra "Apagando {i} de {n}…".
    - No fim o diálogo fecha. O rodapé mostra "{freed} liberados." e, quando houver recusas, "{k} não apagadas", seguido de uma lista curta com o item e o motivo.
    - Os itens apagados somem da lista. As recusas por "regra não vale mais" ou "não existe mais" somem também. As outras ficam.
- **Teste:**
  - `npx tsc -b` passa e o detector do impeccable não aponta nada.
  - No navegador, a tela funciona como hoje.
  - No app, a varredura real enche a planta (verificado na task 9).

---

## 9. Verificação contra o PRD

- **Escopo:** rodar o app de verdade e conferir os quatro objetivos.
- **Contexto:** a seção Objetivos do PRD. A Estratégia de teste do techspec define que o frontend é verificado à mão.
- **Skills:** `run`, para lançar o app.
- **Regras:** nenhuma exclusão real sem a confirmação explícita do usuário. Esta task só apaga pastas de teste que ela mesma criar.
- **Steps:**
  - Rodar `npm run tauri dev` e medir quanto tempo levam os primeiros resultados e a varredura completa de `~` e de `~/Documentos/development`.
  - Rodar o teste ignorado de desempenho.
  - **Zero falso positivo:** listar os artefatos achados em `~/Documentos/development` para você auditar.
  - **Número confere:** criar numa pasta temporária, dentro de `~`, um projeto falso com um `node_modules` de alguns MB e hardlinks. Apagar pelo app e comparar o `df` de antes e depois com o "liberado".
  - **Espaço recuperado:** fica para a sua primeira limpeza real. O número vai para o Histórico do PRD no `sdd-review`.
- **Teste:** os números medidos ficam no resumo final. Não gera commit, a não ser que a verificação revele correção, e aí a correção entra com o próprio commit.
