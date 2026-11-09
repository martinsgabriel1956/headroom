# Techspec: Varredura e limpeza reais

## Resumo executivo

Todo o trabalho com o disco mora no backend Rust do Tauri 2. O frontend React só recebe dados e envia intenções.

A varredura é um percurso paralelo próprio, escrito com `std::fs` e `rayon`. Ele descobre os artefatos pelas regras, não entra neles nem em outros sistemas de arquivos, e entrega cada artefato já medido por um `tauri::ipc::Channel`. É isso que faz os resultados aparecerem durante a varredura.

O backend guarda a última varredura em memória: caminhos, regras e o mapa de hardlinks de cada artefato. A interface só conhece ids. Medir a seleção e apagar são comandos que recebem ids, consultam esse estado e revalidam cada pasta no disco antes de agir. Essa é a fronteira de confiança que o PRD exige.

A decisão principal é calcular o tamanho real a partir de blocos alocados e da contagem de hardlinks (`st_blocks`, `st_nlink`, `st_ino`). Um arquivo só conta quando todas as suas ligações estão dentro do conjunto que seria apagado. É o mesmo princípio do `du`, levado até a seleção.

## Arquitetura

### Visão de componentes

**Backend (`src-tauri/src/`)**

- `rules.rs`: as regras do RF10. Uma função pura recebe o nome da pasta e o caminho do pai e diz se aquilo é um artefato, qual regra o aprovou e o texto de evidência. Ela é chamada na descoberta e de novo na revalidação.
- `measure.rs`: mede uma pasta.
  - Soma o tamanho aparente e os blocos dos arquivos com uma única ligação.
  - Para arquivos com várias ligações, guarda o inode, quantas ligações caem dentro da pasta, o total de ligações e os blocos.
  - Registra também a modificação mais recente.
  - `real_of(conjunto)` combina vários desses mapas para responder quanto um conjunto de artefatos libera.
- `scan.rs`: o percurso.
  - Desce a raiz em paralelo e não segue links simbólicos.
  - Ignora entradas de outro `st_dev`.
  - Quando reconhece um artefato, não desce nele para procurar outros e entrega a pasta para medição em paralelo.
  - Fora dos artefatos, registra os diretórios com arquivos modificados nos últimos 7 dias, para o RF14.
  - Descobre o projeto de cada artefato subindo até o `.git` mais próximo, com cache por diretório.
  - Conta as pastas puladas por falta de permissão.
  - Obedece a um sinal de cancelamento.
- `delete.rs`: revalida e apaga.
  - Para cada id, confere que o caminho continua dentro da raiz e no mesmo `st_dev`.
  - Confere com `symlink_metadata` que ainda é diretório e não link.
  - Roda a mesma regra de `rules.rs` sobre o estado atual do disco.
  - Só então chama `std::fs::remove_dir_all`, que não segue links desde o Rust 1.58.1.
  - Emite progresso por item e devolve um relatório.
- `lib.rs`: registra o plugin de diálogo e o plugin de log, o estado compartilhado e os comandos.

**Frontend (`src/`)**

- `scan.ts` (novo):
  - Um hook `useHeadroom()` que encapsula os comandos: varrer ao montar, trocar a raiz pelo diálogo, reescanear, medir a seleção e apagar.
  - Mantém o estado da varredura: projetos e artefatos, andamento, resumo final e relatório de exclusão.
  - Fora do Tauri, no navegador do `npm run dev`, devolve os dados de `data.ts` para que a tela continue editável sem o backend.
- `App.tsx`:
  - Troca `useState(SAMPLE)` pelo hook.
  - Some o selo de dados de exemplo.
  - O botão da raiz chama o diálogo.
  - O cabeçalho da planta mostra andamento e resumo, numa região `aria-live`.
  - O total do botão Limpar vem de `measure`.
  - A confirmação chama `delete` e mostra as falhas.
  - A planta e a lista não mudam de estrutura.
- `data.ts`: os tipos `Artifact` e `Project` ganham os campos que o backend manda. `SAMPLE` fica só para o modo navegador.

**Fluxo de dados**

```
App monta → scan(root?) ─Channel─▶ Started → Artifact* → Done{resumo, projetos ativos}
seleção muda → measure(ids) → bytes reais (debounce de 150 ms)
confirma → delete(ids) ─Channel─▶ Deleting{id} / Deleted{id} / Refused{id, motivo} → Report
```

O estado da varredura fica num `Mutex<Option<Scan>>` gerenciado pelo Tauri. Uma nova varredura incrementa um contador de geração (`AtomicU64`). O percurso anterior confere esse contador e para sozinho. `measure` e `delete` recusam ids de uma geração antiga.

## Design de implementação

### Interfaces principais

```rust
// rules.rs
pub struct Rule { pub kind: &'static str, pub evidence: &'static str }
pub fn match_rule(name: &OsStr, dir: &Path) -> Option<&'static Rule>;

// measure.rs
pub type Inode = (u64, u64); // (st_dev, st_ino)
pub struct Measured { pub apparent: u64, pub single: u64, pub newest: SystemTime,
                      pub linked: HashMap<Inode, (u32 /*aqui*/, u64 /*nlink*/, u64 /*bytes*/)> }
pub fn measure(dir: &Path, cancel: &dyn Fn() -> bool) -> io::Result<Measured>;
pub fn real_of<'a>(set: impl IntoIterator<Item = &'a Measured>) -> u64;

// scan.rs
pub fn scan(root: &Path, recent_days: u64, cancel: &(dyn Fn() -> bool + Sync),
            found: &(dyn Fn(Found) + Sync)) -> ScanStats;

// delete.rs
pub fn revalidate(root: &Path, dev: u64, found: &Found) -> Result<(), Refusal>;

// lib.rs — comandos Tauri
#[tauri::command] async fn scan(root: Option<String>, on_event: Channel<ScanEvent>, ...) -> Result<(), String>;
#[tauri::command] fn measure(generation: u64, ids: Vec<u32>, ...) -> Result<u64, String>;
#[tauri::command] async fn delete(generation: u64, ids: Vec<u32>, on_event: Channel<DeleteEvent>, ...) -> Result<DeleteReport, String>;
```

Do lado do TypeScript, `useHeadroom()` devolve `{ root, projects, status, summary, chooseRoot(), rescan(), measure(ids), remove(ids) }`. O `invoke` e o `Channel` vêm de `@tauri-apps/api/core`, e o `open({ directory: true })` vem de `@tauri-apps/plugin-dialog`.

### Modelos de dado

Os eventos são serializados com `#[serde(tag = "type")]` e chegam ao TypeScript como uniões discriminadas.

```ts
type ScanEvent =
  | { type: 'started'; generation: number; root: string }
  | { type: 'artifact'; artifact: ArtifactDto }
  | { type: 'progress'; dirs: number }                       // no máximo a cada 100 ms
  | { type: 'done'; millis: number; skipped: number; active: { project: string; days: number }[] }
type ArtifactDto = { id: number; project: string /* caminho absoluto do projeto */; kind: string; rel: string;
                     rule: string; real: number; apparent: number; days: number }
type DeleteEvent = { type: 'deleting' | 'deleted'; id: number } | { type: 'refused'; id: number; reason: string }
type DeleteReport = { freed: number; removed: number[]; refused: { id: number; reason: string }[] } // removed: ids que a tela tira (apagados, apagados em parte ou que já não existiam)
```

- **`real` por artefato:** os blocos dos arquivos com uma ligação, mais os arquivos com várias ligações cujas ligações estão todas dentro dele, mais os blocos dos próprios diretórios.
- **`freed` no relatório:** é o `real_of` dos itens efetivamente apagados, calculado antes de apagar. Não é a diferença do `statvfs`, que oscila com outros processos. A comparação com o `df` é a medida do objetivo, não o número da tela.
- **`days` de projeto ativo:** chega só no `done`, porque depende do percurso inteiro. A lista mostra o projeto sem o selo até lá.
- **Projeto:** a chave `project` é o caminho absoluto da pasta do projeto. O nome exibido é o último componente dele.

### Endpoints de API

Não há API HTTP. A superfície é a dos três comandos Tauri acima.

## Pontos de integração

- **Diálogo nativo:** `tauri-plugin-dialog` 2.x, com a permissão `dialog:allow-open` em `capabilities/default.json`. Se o usuário cancelar, o diálogo devolve `null` e a raiz não muda.
- **Pasta pessoal:** vem de `$HOME`. Sem `$HOME`, o comando `scan` devolve um erro, e a tela o mostra no lugar da planta.
- **Erros de E/S:**
  - Na varredura, `PermissionDenied` conta como pasta pulada. Outros erros por entrada também contam como pulados e vão para o log, sem interromper.
  - Na exclusão, uma falha do `remove_dir_all` no meio do caminho vira `Refused` com o motivo "apagado em parte: …". O item sai da tela, porque a pasta não está mais íntegra, e o `freed` não o conta.

## Estratégia de teste

**Unitário (Rust, `cargo test`).** Os testes montam árvores reais num diretório temporário único sob `std::env::temp_dir()`, sem mocks.

- **`rules`:** cada linha da tabela do RF10, com e sem a evidência. `dist` e `build` nunca casam.
- **`measure`:**
  - Um arquivo com hardlink para fora da pasta não conta no real.
  - Um hardlink entre dois arquivos da mesma pasta conta uma vez.
  - `real_of` de duas pastas que compartilham inodes conta cada inode uma vez.
  - O aparente soma os tamanhos.
- **`scan`:**
  - Não entra num `node_modules` aprovado: um `node_modules` aninhado não aparece.
  - Não segue symlink.
  - Varre pastas ocultas.
  - Acha o projeto pelo `.git` mais próximo.
  - Marca como ativo o projeto com arquivo novo fora do artefato, e não marca quando o único arquivo novo está dentro do artefato.
  - Conta pastas sem permissão como puladas.
- **`delete`:**
  - Recusa quando o manifesto sumiu depois da varredura.
  - Recusa quando a pasta virou symlink.
  - Recusa quando o caminho saiu da raiz.
  - Apaga quando tudo confere.
  - Uma recusa não impede os outros itens.

**Integração.**
- Um teste chama o fluxo do comando `scan` sobre uma árvore de exemplo, coletando os eventos, e confere a ordem: `started`, depois artefatos, depois `done`.
- Um teste com `#[ignore]` mede o tempo sobre `~/Documentos/development`, para verificar a meta de desempenho à mão (`cargo test -- --ignored`).

**Frontend.** O projeto não tem framework de teste, e a lógica nova é encanamento de estado. A verificação é manual no app rodando: `npm run tauri dev` sobre a pasta real, conferindo os objetivos do PRD. O teste do `treemap` continua.

**E2E.** Nenhum nesta feature.

## Sequenciamento

1. **`rules.rs`:** é puro, sem dependências, e a revalidação também o usa.
2. **`measure.rs`:** depende só do disco.
3. **`scan.rs`:** usa os dois anteriores.
4. **`delete.rs`:** usa `rules.rs` e o estado de varredura.
5. **`lib.rs`:**
   - Comandos, estado, geração e plugins.
   - Configuração do app: nome Headroom, janela de 1280×800 com mínimo de 1024×680, identificador e permissão do diálogo.
6. **`scan.ts` e `App.tsx`:** a ligação com o frontend.
7. **Verificação manual:** contra os quatro objetivos do PRD.

Os passos 1 a 4 não precisam do Tauri e são testados só com `cargo test`.

## Observabilidade

Usa o `tauri-plugin-log` que o scaffold já registra em debug. Linhas de log:
- **Fim de varredura:** raiz, tempo, diretórios lidos, artefatos, pulados.
- **Por exclusão:** id, caminho, resultado, bytes.
- **Recusa:** motivo.

Não há métrica remota, porque nada sai da máquina.

## Considerações técnicas

### Decisões principais

- **Percurso próprio com `std::fs` e `rayon`, em vez de `jwalk`.** O `jwalk` foi o plano inicial. A versão atual no crates.io se declara descontinuada e indica o `dua-core`. O percurso precisa de três cortes próprios: não descer em artefato, não trocar de `st_dev` e não seguir link. Os três são triviais num `read_dir` recursivo com `rayon::scope`, e o `rayon` é uma dependência madura e pequena. O `dua-core` foi descartado por trazer a lógica de um analisador inteiro para usar só o percurso.
- **Estado no backend, ids no frontend.** É o que permite cumprir a fronteira de confiança do PRD sem validar caminhos vindos da interface. Também permite medir a seleção com os mapas de hardlink, que ficam no backend.
- **`Channel` em vez de eventos globais (`emit`).** O `Channel` é ligado a uma chamada, preserva a ordem e é o mecanismo que o Tauri 2 recomenda para streaming. Eventos globais misturariam varreduras concorrentes.
- **Real por blocos e ligações.** Os blocos dão o espaço alocado, inclusive em arquivos esparsos. A contagem de ligações dentro do conjunto resolve o pnpm sem caso especial. A alternativa de comparar o `statvfs` antes e depois foi descartada como número da tela, por oscilar com a atividade do sistema.
- **Atividade sem segundo percurso.** O percurso principal já visita todo arquivo fora de artefato. Ele guarda só os diretórios com arquivo recente, o que é pouca memória, e no fim agrega por projeto. A alternativa de percorrer cada projeto de novo foi descartada por dobrar o custo em `~`.
- **`.git` não é percorrido.** É um banco de objetos onde nenhuma regra casa. Entrar nele custaria tempo em todo repositório, e uma operação de git não é edição do projeto para o RF14. É a única pasta oculta ignorada. O RF7 continua valendo para as demais.
- **Seleção medida no backend com debounce de 150 ms.** O total exato depende de interseção de inodes. Somar os `real` no frontend seria quase sempre igual, mas erraria no caso do RF19.
- **Navegador continua com dados de exemplo.** Permite seguir ajustando a tela com o Vite puro e não custa nada no app real.

- **A planta agrupa os projetos pequenos.** O desenho é feito no frontend: no máximo 24 cômodos, só os com pelo menos 1% do total, e o resto num cômodo "+N projetos" não selecionável. O backend continua mandando tudo. O corte foi descoberto na primeira varredura real de `~`, que achou 176 projetos.
- **O frontend é verificado à mão, e a exclusão contra o `df` por teste.** Numa sessão Wayland, o `xdotool` não consegue clicar no app. A conferência do PRD foi feita pelo teste ignorado `freed_matches_df`, que roda o mesmo `run_delete` do comando. A lista de auditoria sai do teste ignorado `timing`.
- **Aviso de lint aceito.** O `oxlint` aponta `set-state-in-effect` em `useHeadroom`, no efeito que dispara a varredura ao montar. É sincronização com um sistema externo (o comando `scan`), e o reset de estado precisa acontecer junto. O reset da seleção em `App.tsx` saiu do efeito e foi para os handlers de Escanear e de troca de raiz.

### Riscos conhecidos

- **`~` é grande.** Com cache frio, a varredura pode levar dezenas de segundos. A mitigação é o streaming com medição paralela. Ainda falta medir o tempo real de `~` na máquina do usuário. O RF5 mostra o andamento.
- **Memória dos mapas de hardlink.** Um `node_modules` do pnpm pode ter dezenas de milhares de arquivos ligados. São cerca de 40 bytes por entrada, alguns MB no pior caso razoável. Se crescer demais, o mapa pode guardar só os inodes que tocam outro artefato.
- **Pastas de ferramentas em `~`.** O usuário aceitou o risco no PRD (RF7). Nada técnico mitiga além da confirmação explícita.
- **Corrida entre varredura e exclusão.** A revalidação cobre o caso da pasta que mudou. Um arquivo novo criado dentro do artefato entre a medição e a exclusão faz o `freed` subestimar um pouco. Isso é aceitável dentro da margem de 5%.
- **`remove_dir_all` parcial.** Tratado como recusa com motivo (ver Pontos de integração).
- **CSP desligada.** `tauri.conf.json` mantém `csp: null`, como veio do scaffold. O app só carrega o próprio frontend e não usa rede, mas vale ligar uma CSP restrita antes da distribuição (v3).
- **Montagens FUSE lentas.** São evitadas pelo corte de `st_dev`. Um `stat` na raiz de uma montagem travada ainda pode demorar, e é a única chamada feita nela.

### Conformidade com regras do projeto

O projeto não tem `CLAUDE.md` nem diretório de regras. As convenções que se aplicam vêm dos documentos do próprio repositório:
- **PRODUCT.md:** a exclusão permanente e as regras validadas são respeitadas. O `jwalk` citado na Stack foi trocado por `rayon` (ver Decisões principais). O PRODUCT.md precisa ser atualizado junto.
- **DESIGN.md:** a tela não muda de estrutura. Os estados novos usam os tokens existentes. O andamento usa a voz de anotação da planta. As falhas usam ink e graphite, não brick.
- **Desvios:** nenhum.

### Conformidade com skills disponíveis

- **`sdd-tasks` e `sdd-review`:** continuam o pipeline.
- **`ponytail`** (plugin do usuário): orienta a implementação para a solução mínima, por exemplo `std` antes de crate nova, e deixa um teste por lógica não trivial.
- **`impeccable`:** só se a tela ganhar mudança visual além dos estados descritos. O hook de design continua ativo nos arquivos de UI.

### Arquivos relevantes

- **Backend:**
  - `src-tauri/Cargo.toml`: nome do pacote e dependências `rayon`, `tauri-plugin-dialog` e `libc`. O `libc` só entra se `MetadataExt` não bastar, e deve bastar.
  - `src-tauri/tauri.conf.json` e `src-tauri/capabilities/default.json`.
  - `src-tauri/src/lib.rs`, `rules.rs`, `measure.rs`, `scan.rs` e `delete.rs`.
- **Frontend:** `src/scan.ts`, `src/App.tsx` e `src/data.ts`.
- **Documentos:** `PRODUCT.md` (troca do `jwalk` na Stack).

## Histórico

**2026-11-09**. Documento criado.

**2026-11-09**. Fechamento (`sdd-review`). Mudanças registradas:
- `DeleteReport` ganhou `removed`.
- `ArtifactDto` ficou sem `projectPath`, porque `project` já é o caminho absoluto.
- A planta agrupa os projetos pequenos.
- A exclusão contra o `df` passou a ser verificada pelo teste ignorado `freed_matches_df`.
- O aviso de lint foi aceito.
- O risco da CSP ficou anotado.

O resto saiu como planejado: o percurso com `rayon` e cortes de link, disco, artefato e `.git`; o estado por geração; o `Channel`; e o tamanho real por blocos e ligações.
