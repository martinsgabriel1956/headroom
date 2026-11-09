# PRD: Varredura e limpeza reais

## Visão geral

O Headroom já tem a tela principal (a Planta Baixa) funcionando com dados de exemplo. Falta o que faz dele uma ferramenta: ler o disco de verdade, achar os artefatos regeneráveis dos projetos e apagá-los com segurança.

O problema é de quem tem muitos projetos locais e vê o disco encher de `node_modules`, `target`, `.venv` e caches que se regeneram com um install ou build. Apagar isso na mão dá trabalho, porque está espalhado por dezenas de pastas, e dá medo, porque um `rm -rf` no lugar errado leva código-fonte. As ferramentas comuns também mentem no número: somam o tamanho aparente, e no `node_modules` do pnpm, que é quase todo hardlink, prometem gigabytes que não voltam.

Esta feature entrega o ciclo completo numa raiz de projetos: varrer, mostrar na planta e na lista com o espaço que realmente volta, e apagar de vez o que o usuário escolher, revalidando cada pasta antes.

## Objetivos

- **Zero falso positivo.** Nada que não seja regenerável aparece na lista, e nada fora dos artefatos detectados é apagado. Medida: auditoria manual da primeira varredura real de `~/Documentos/development`, item por item. Fora dela, em pastas ocultas de ferramentas, o risco é aceito (RF7).
- **O número confere.** O espaço liberado que o Headroom informa bate com a diferença de espaço livre do sistema antes e depois da limpeza, com margem de até 5%.
- **Varredura rápida.** Com `~/Documentos/development` como pasta de referência, os primeiros resultados aparecem em até 1 segundo e a varredura termina em até 10 segundos, com o cache do sistema de arquivos aquecido. Com o cache frio pode passar disso. A varredura de `~` inteira não tem meta de tempo total, só a de primeiros resultados em até 1 segundo.
- **Espaço recuperado.** A primeira limpeza real recupera uma quantidade relevante de espaço. O número fica anotado no Histórico como linha de base.

## Histórias de usuário

- Como desenvolvedor com o disco quase cheio, eu quero abrir o app e já ver onde está o espaço recuperável, para decidir rápido o que limpar.
- Como desenvolvedor, eu quero ver o espaço real de cada artefato, para não escolher um `node_modules` do pnpm achando que vou recuperar centenas de MB.
- Como desenvolvedor, eu quero ver que projetos mexi nos últimos dias, para não apagar o build do projeto em que estou trabalhando e esperar um build completo depois.
- Como desenvolvedor, eu quero trocar a pasta varrida por outra, para limpar um diretório fora do padrão sem mudar configuração.
- Como desenvolvedor, eu quero confirmar uma lista exata antes de apagar, e saber o que não foi apagado e por quê, para confiar no que a ferramenta faz.
- Caso de borda: se uma pasta mudou entre a varredura e a confirmação (foi apagada, virou link, perdeu o manifesto), ela não é apagada, e eu sou avisado.
- Caso de borda: se parte da varredura não tem permissão de leitura, a varredura continua e me diz quantas pastas foram puladas.

## Funcionalidades principais

### 1. Varredura

Percorre a raiz escolhida e encontra os artefatos. Importa porque é a única fonte dos números que a tela mostra.

- **RF1.** Ao abrir, o app varre automaticamente a raiz padrão, a pasta pessoal do usuário (`~`).
- **RF2.** O usuário pode escolher outra raiz pelo seletor de pastas nativo do sistema. A escolha vale só para a sessão; o app não lembra dela ao reabrir.
- **RF3.** O botão Escanear refaz a varredura da raiz atual, descartando a seleção anterior.
- **RF4.** Os artefatos aparecem na planta e na lista à medida que são encontrados, sem esperar o fim da varredura.
- **RF5.** A tela indica que a varredura está em andamento e, ao terminar, mostra quantos artefatos achou e quanto tempo levou.
- **RF6.** Pastas sem permissão de leitura são puladas sem interromper a varredura. Ao fim, a tela informa quantas foram puladas.
- **RF7.** A varredura não segue links simbólicos, não sai da raiz e não entra em outros sistemas de arquivos montados dentro dela, como discos externos, pastas de rede ou montagens FUSE. Pastas ocultas (que começam com ponto) são varridas como qualquer outra. Isso inclui pastas de ferramentas como `~/.vscode`, `~/.cache` e `~/.local`. Artefatos dentro delas aparecem na lista. Um exemplo é o `node_modules` de uma extensão do editor, que a regra aprova mas que o usuário não regenera com um install.
- **RF8.** Depois de reconhecer um artefato, a varredura não entra nele procurando outros.
- **RF9.** Nada vem pré-selecionado. O usuário escolhe tudo.

### 2. Detecção por regras

Cada artefato só é listado se uma regra confirmar que ele é regenerável. Importa porque é a garantia de zero falso positivo.

- **RF10.** Regras da primeira versão:

  | Pasta | Condição para contar como artefato |
  |---|---|
  | `node_modules` | existe um `package.json` na mesma pasta pai |
  | `target` | existe um `Cargo.toml` na mesma pasta pai |
  | `.venv`, `venv` | existe um `pyvenv.cfg` dentro dela |
  | `__pycache__`, `.pytest_cache` | sempre (só contêm cache) |
  | `.next`, `.turbo` | existe um `package.json` na mesma pasta pai |
  | `.gradle` | existe um `build.gradle`, `build.gradle.kts` ou `settings.gradle` na mesma pasta pai |

- **RF11.** `dist/` e `build/` não são detectados, porque às vezes são versionados.
- **RF12.** Cada artefato mostra a evidência da regra que o aprovou (por exemplo, "Cargo.toml ao lado").
- **RF13.** O projeto de um artefato é a pasta mais próxima acima dele que tem um repositório git. Sem git no caminho até a raiz, o projeto é a pasta que contém o artefato. O artefato é exibido pelo caminho relativo ao projeto (por exemplo, `src-tauri/target`).
- **RF14.** Um projeto é **ativo** quando algum arquivo dele, fora dos artefatos, foi modificado nos últimos 7 dias. A tela marca o projeto como ativo e mostra há quanto tempo foi a modificação.
- **RF15.** Cada artefato mostra há quanto tempo foi modificado pela última vez.

### 3. Tamanho real e aparente

Os números precisam ser verdadeiros. Importa porque é o diferencial do produto.

- **RF16.** O tamanho **real** de um artefato é o espaço em disco que volta se ele for apagado. Arquivos com vários hardlinks contam uma vez só, e não contam se alguma cópia do link continua existindo fora do conjunto apagado.
- **RF17.** O tamanho **aparente** é a soma dos tamanhos dos arquivos, como o gerenciador de arquivos mostra.
- **RF18.** A alternância Real/Aparente da tela usa esses dois números. Total selecionado, botão Limpar, confirmação e mensagem final usam sempre o real.
- **RF19.** O real de uma seleção com vários artefatos considera os hardlinks entre eles. Se dois artefatos selecionados compartilham arquivos, esses arquivos contam uma vez.

### 4. Exclusão segura

Apaga de vez o que foi confirmado e mais nada. Importa porque é a ação irreversível do produto.

- **RF20.** A exclusão só aceita itens da última varredura, nunca um caminho qualquer.
- **RF21.** Antes de apagar cada item, o app confirma de novo que ele continua dentro da raiz, que não virou link simbólico e que a regra que o aprovou ainda vale. Se alguma verificação falhar, o item não é apagado.
- **RF22.** A exclusão é permanente e não passa pela lixeira. A confirmação diz isso.
- **RF23.** Uma falha em um item não impede os outros. No fim, a tela mostra quanto foi liberado de fato e lista os itens que não foram apagados, com o motivo de cada um.
- **RF24.** Durante a exclusão, a tela mostra o progresso e não aceita uma segunda exclusão ao mesmo tempo.
- **RF25.** Os itens apagados saem da planta e da lista, e o rodapé mostra o espaço liberado, como a tela já faz com os dados de exemplo.
- **RF26.** Selecionar um projeto ativo continua permitido. A confirmação avisa, como já faz hoje.

## Experiência do usuário

A tela não muda de estrutura: é a Planta Baixa descrita no DESIGN.md, com a planta no topo, a lista agrupada por projeto e o rodapé com a linha de cota e o botão Limpar. O que muda:

- **Selo de dados de exemplo:** o selo "Dados de exemplo" sai do app.
- **Botão da pasta raiz:** passa a abrir o seletor nativo.
- **Varredura em andamento:** a planta e a lista vão se enchendo, e o cabeçalho da planta mostra o andamento e, no fim, a contagem, o tempo e as pastas puladas.
- **Estado vazio:** uma raiz sem nenhum artefato mostra a planta vazia com a mensagem que já existe.
- **Muitos projetos:** a planta mostra no máximo 24 projetos, só os que têm pelo menos 1% do total. Os outros viram um único cômodo "+N projetos", que não é selecionável. Todos continuam na lista, onde podem ser selecionados.
- **Falhas de exclusão:** quando algum item falha, o resultado final mostra quais foram e por quê, com a mesma voz do resto da tela, sem alarme.
- **Acessibilidade:** o andamento e o resultado da varredura e da exclusão são anunciados para leitores de tela. Todos os controles continuam acessíveis pelo teclado.

## Restrições técnicas de alto nível

- **Plataforma:** app desktop para Linux. Outros sistemas ficam fora desta versão.
- **Sem privilégios elevados:** o app roda sem pedir senha de administrador e só toca no que o usuário pode ler e apagar.
- **Tudo local:** nenhum dado sai da máquina e o app não usa rede.
- **Fronteira de confiança:** a interface nunca decide o que é apagado por um caminho. A decisão final é sempre revalidada fora dela (RF20 e RF21).
- **Responsividade:** a interface continua respondendo durante a varredura e a exclusão.
- **Metas de desempenho:** as da seção Objetivos.

## Fora de escopo

- **Aba Docker** (imagens órfãs, build cache): prevista para a v2.
- **Regras personalizadas** por arquivo de configuração.
- **`dist/` e `build/`:** nem como opção.
- **Raízes:** varrer várias ao mesmo tempo, ou lembrar a raiz escolhida entre sessões.
- **Automação:** limpeza agendada ou automática.
- **Lixeira e desfazer.**
- **Plataformas e distribuição:** Windows, macOS, e o empacotamento (.deb, AppImage), previsto para a v3.
- **Pré-seleção** de qualquer tipo.

## Histórico

**2026-11-09**. Documento criado. Inclui a exclusão real, que no PRODUCT.md estava na v1. Por decisão do usuário, a v0 já entrega varrer e apagar.

**2026-11-09**. Raiz padrão mudou de `~/Documentos/development` para `~` (RF1). Decisão do usuário: pastas ocultas são varridas sem exceção (RF7). O usuário aceitou o risco de artefatos de ferramentas aparecerem na lista. A meta de zero falso positivo continua medida em `~/Documentos/development`. A meta de tempo total passou a valer só para essa pasta de referência.

**2026-11-09**. Decisão do usuário durante o techspec: a varredura fica no sistema de arquivos da raiz e não entra em outras montagens (RF7). Isso evita travar em pastas de rede e FUSE, e garante que o espaço liberado é do disco mostrado.

**2026-11-09**. Fechamento (`sdd-review`). A primeira varredura real de `~` achou 176 projetos, e a legenda de chaves esmagava a planta. A planta passou a agrupar os projetos pequenos num cômodo "+N projetos" (Experiência do usuário). Resultados medidos contra os objetivos, com cache aquecido:
- **Desempenho:** `~/Documentos/development` em 0,17 s e `~` em 0,32 s, ambos em build de release. O primeiro resultado sai em menos de 2 ms.
- **Número confere:** numa pasta de teste, o app informou 1.572.876.288 bytes liberados e o `df` mudou 1.572.818.944 (0,004%). O arquivo ligado por hardlink a outro fora da pasta não foi contado.
- **Auditoria:** 30 artefatos em `~/Documentos/development`, todos regeneráveis pela lista do teste de tempo.
- **Pendente:** o espaço recuperado na primeira limpeza real do usuário ainda não foi registrado.
