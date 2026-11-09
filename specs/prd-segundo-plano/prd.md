# PRD: Modo em segundo plano

## Visão geral

Hoje o Headroom só ajuda quando o usuário lembra de abri-lo. Normalmente isso acontece tarde, quando o disco já encheu, um build falhou por falta de espaço ou o sistema começou a reclamar. Os artefatos regeneráveis crescem aos poucos, a cada `npm install` ou `cargo build`, e ninguém percebe até virar problema.

Esta feature mantém o Headroom rodando discretamente na bandeja do sistema. Ele avisa em dois momentos: quando o disco fica cheio e quando os artefatos acumulados passam a valer uma limpeza. O usuário continua decidindo e confirmando toda exclusão na janela; o segundo plano só observa e avisa.

## Objetivos

- **Aviso antes do aperto.** Quando o espaço livre cai abaixo do limite, a notificação chega em até 15 minutos.
- **Sem ruído.** Cada aviso sai uma vez quando a situação começa e só volta a sair depois que ela se resolve e acontece de novo. Medida: nenhuma notificação repetida para a mesma situação contínua.
- **Invisível quando ocioso.** Entre uma checagem e outra, o app não usa CPU perceptível. Uma varredura periódica não pode deixar o computador lento a ponto de o usuário notar.
- **Uso real.** O usuário deixa o Headroom na bandeja por pelo menos uma semana, em vez de fechá-lo de vez. É uma avaliação qualitativa, feita pelo próprio usuário.

## Histórias de usuário

- Como desenvolvedor, eu quero fechar a janela sem encerrar o Headroom, para ele continuar me avisando.
- Como desenvolvedor, eu quero ser avisado quando o disco estiver ficando cheio, antes de um build falhar por falta de espaço.
- Como desenvolvedor, eu quero saber quando já dá para recuperar uma quantidade que vale a pena, para limpar no momento certo e não por pânico.
- Como desenvolvedor, eu quero clicar no aviso e cair direto na planta com a varredura mais recente, para decidir o que apagar.
- Como desenvolvedor, eu quero encerrar o Headroom de vez quando quiser, sem caçar o processo.
- Caso de borda: se o disco continua cheio por dias, eu recebo um único aviso, não um por checagem.
- Caso de borda: se eu estou com a janela aberta selecionando itens, nenhuma varredura automática troca a lista na minha frente.

## Funcionalidades principais

### 1. Bandeja e ciclo de vida

Mantém o app vivo sem a janela. Importa porque é o que torna os avisos possíveis.

- **RF1.** Enquanto o app roda, ele mostra um ícone na bandeja do sistema, com a logo do Headroom.
- **RF2.** Fechar a janela só a esconde. O app continua rodando na bandeja.
- **RF3.** Minimizar a janela funciona como em qualquer app, e o Headroom continua rodando.
- **RF4.** Na primeira vez que a janela é fechada, uma notificação explica que o Headroom continua na bandeja e como sair. Esse aviso não se repete depois.
- **RF5.** O ícone da bandeja leva à janela e a traz para a frente. No Linux, o clique no ícone sempre abre o menu, então o caminho é o item "Abrir Headroom" (RF6).
- **RF6.** O menu da bandeja tem três itens:
  - **Abrir Headroom:** mostra a janela.
  - **Escanear agora:** mostra a janela e começa uma varredura.
  - **Sair:** encerra o app de vez.
- **RF7.** Abrir o Headroom de novo enquanto ele já está rodando não cria uma segunda cópia. A janela da cópia que já roda aparece.

### 2. Aviso de disco cheio

Avisa antes de o espaço acabar. Importa porque é o momento em que o usuário mais precisa do app.

- **RF8.** O app checa o espaço livre do disco da pasta varrida a cada 15 minutos. A pasta é a raiz atual, que por padrão é `~`.
- **RF9.** O disco está **cheio** quando o espaço livre fica abaixo de 10% do total **ou** abaixo de 20 GB, o que acontecer primeiro.
- **RF10.** Quando o disco passa a ficar cheio, sai uma notificação com o espaço livre atual e, se já se souber, quanto dá para recuperar. Exemplo: "Disco quase cheio: 14 GB livres. O Headroom acha 16,4 GB para limpar."

### 3. Varredura periódica

Mantém o número de espaço recuperável atualizado sem o usuário pedir. Importa porque transforma o app de reativo em preventivo.

- **RF11.** A cada 6 horas, o app varre a raiz atual sozinho, com as mesmas regras da varredura manual.
- **RF12.** A varredura automática só roda com a janela escondida. Se a janela estiver visível na hora marcada, ela espera a janela ser escondida.
- **RF13.** Quando o espaço recuperável passa a ser de 5 GB ou mais, sai uma notificação com o total. Exemplo: "16,4 GB para limpar em 176 projetos."
- **RF14.** Ao abrir a janela, ela mostra o resultado da varredura automática mais recente, sem varrer de novo.
- **RF15.** A varredura automática nunca apaga nada. Toda exclusão continua passando pela confirmação da janela.

### 4. Regras de notificação

Evita que os avisos virem ruído. Importa porque um aviso ignorado não serve para nada.

- **RF16.** Cada tipo de aviso (disco cheio, espaço recuperável) sai uma vez quando a condição começa. Ele só pode sair de novo depois que a condição deixa de valer e volta a valer.
- **RF17.** Clicar numa notificação abre a janela do Headroom, como no RF5.
- **RF18.** Os limites são fixos nesta versão: 10% e 20 GB para o disco cheio, 6 horas entre varreduras, 5 GB para o espaço recuperável e 15 minutos entre checagens do disco.

## Experiência do usuário

- **A tela não muda.** A janela continua sendo a Planta Baixa do DESIGN.md. O que muda é o ciclo de vida: o fechar esconde, a bandeja reabre.
- **Ícone da bandeja:** é a logo do Headroom, o medidor de disco, nos tamanhos da bandeja.
- **Texto das notificações:** curto, na voz do app, com o número na frente e sem alarme, seguindo a regra de não parecer antivírus do PRODUCT.md. Nunca "seu sistema está lento!".
- **Primeira vez que fecha:** a notificação do RF4 evita a surpresa de "fechei e continua rodando".
- **Acessibilidade:** os itens do menu da bandeja e as notificações usam os mecanismos do sistema, que os leitores de tela já anunciam.

## Restrições técnicas de alto nível

- **Plataforma:** Linux. A bandeja depende do suporte do ambiente de desktop. O GNOME, por exemplo, precisa da extensão de AppIndicator. Sem bandeja disponível, fechar a janela deve encerrar o app, como hoje, para nunca deixar um processo invisível sem como sair.
- **Notificações:** as do sistema (padrão freedesktop). Nada de janela própria pulando na tela.
- **Tudo local:** continua sem rede e sem privilégios elevados.
- **Exclusão:** as regras de exclusão segura do PRD de varredura e limpeza continuam valendo sem mudança (RF20 a RF23 daquele PRD).
- **Consumo:** o app fica leve quando ocioso, e a varredura automática não pode competir com o trabalho do usuário.

## Fora de escopo

- **Iniciar com o sistema:** abrir escondido ao entrar na sessão. O usuário não escolheu isso agora.
- **Configurações:** tela de preferências para mudar limites, intervalos ou desligar avisos.
- **Limpeza automática** de qualquer tipo.
- **Discos e Docker:** monitorar outros discos além do da raiz atual, e avisos sobre Docker (v2).
- **Badge no ícone,** com o número de GB no próprio ícone da bandeja.
- **Outros sistemas:** Windows e macOS.

## Histórico

**2026-11-09**. Documento criado. O usuário escolheu:
- bandeja, aviso de disco cheio e varredura periódica, sem iniciar com o sistema;
- limites fixos;
- avisos só quando a situação muda;
- o clique abre a janela.

Ficaram por conta da decisão do documento, e o usuário pode mudar:
- a checagem de espaço livre a cada 15 minutos;
- a varredura automática só com a janela escondida.

**2026-11-09**. Ajuste vindo do techspec: no Linux, o AppIndicator não entrega o clique no ícone ao app, e o clique sempre abre o menu. O RF5 passa a ser cumprido pelo item "Abrir Headroom".

**2026-11-09**. Fechamento (`sdd-review`). Todos os RFs estão implementados.
- **Conferidos com o app rodando em intervalos curtos:** RF1, RF2, RF4, RF7 a RF16 e RF18.
- **Pendentes de teste manual:** RF5, RF6 e RF17, que dependem de clicar no ícone da bandeja, no menu e na notificação.
- **Não testado:** o caminho "sem bandeja, o X fecha", porque não há um desktop sem bandeja à mão. Está coberto pela detecção no início do app.
