# Definições {#settings}

![Settings: account, users, single sign-on, agents, assistant, appearance, about](../assets/screenshots/settings-light.png){ loading=lazy }

As Definições contêm o que é administrativo: a sua conta, quem pode iniciar
sessão e como, os tokens que deixam entrar agentes e assistentes, a cópia de
segurança da instância, o tema, e a saúde do servidor. Uma página, uma secção
por tema, com uma barra lateral de âncoras num ecrã largo e uma faixa de chips
num telemóvel. Cada secção pode ser ligada diretamente (`/settings#agents`).

Duas coisas viviam aqui e mudaram-se, porque não são administração:

| À procura de | Agora em | Ligação antiga |
|---|---|---|
| Canais de notificação, horas de silêncio, política de notificação | **Alertas → Notificações** ([página Alertas](alerts.md#notifications)) | `/settings#notifications`, `/settings#notifications-policy` |
| Páginas de estado e anúncios | **Estado** ([Páginas de estado](status-pages.md)) | `/settings#status` |

As ligações antigas continuam a funcionar: reencaminham para o novo local.

## Conta e segurança {#account-security}

Esta é a sua própria conta. A primeira é criada no primeiro arranque, e podem
ser adicionadas mais abaixo. Aqui altera a sua palavra-passe: palavra-passe
atual, nova palavra-passe (pelo menos 12 caracteres; uma frase inteira é mais
segura do que uma palavra complicada), confirmação. Alterá-la termina todas as
outras sessões. As contas que iniciam sessão através do fornecedor de
identidade não têm palavra-passe aqui. **Terminar sessão** termina esta sessão.

O início de sessão tem limite de tentativas por endereço de cliente e por
conta: após cinco tentativas falhadas, cada tentativa seguinte espera mais (30
s, a duplicar, até 5 minutos). Atrás de um proxy inverso, defina
`DUMBMONIT_TRUSTED_PROXIES` para que o limitador veja o endereço real do cliente
em vez do do proxy (veja [Configuração](../reference/configuration.md)). As
sessões duram 30 dias. Uma palavra-passe esquecida repõe-se com
`DUMBMONIT_RESET_PASSWORD=1`: veja as [FAQ](../faq.md#i-lost-the-password).

### Autenticação de dois fatores {#two-factor-authentication}

As contas que iniciam sessão com palavra-passe podem adicionar um segundo fator:
um código de seis dígitos baseado em tempo (TOTP) de uma app autenticadora —
Aegis, FreeOTP, Google Authenticator, 1Password, Bitwarden… **Configurar** pede
a sua palavra-passe, mostra um código QR para ler (ou a chave para escrever), e
ativa o segundo fator quando introduz um primeiro código da app. São então
mostrados uma só vez oito **códigos de recuperação**: guarde-os, cada um
permite-lhe iniciar sessão uma única vez se o telemóvel se perder. O ecrã de
início de sessão pede o código (ou um código de recuperação) depois da
palavra-passe; cinco códigos errados cancelam a tentativa e recomeça pela
palavra-passe, e o limite habitual de tentativas continua a contar entre
tentativas. Um código é aceite uma só vez: o que acabou de usar (incluindo o que
ativou o segundo fator) não o faz iniciar sessão de novo — espere pelo
seguinte.

**Desativar** pede de novo a palavra-passe. Os códigos de recuperação não podem
ser regenerados por si: desative e configure de novo para obter um conjunto
novo. Um administrador pode repor o segundo fator de outro utilizador em
**Utilizadores** (*Reset 2FA*) quando o telemóvel e os códigos se perderam — isso
termina a sessão do utilizador em todo o lado e deixa a palavra-passe como
está. O segredo é guardado cifrado com o segredo da instância; os códigos são
guardados em hash. As contas de início de sessão único recebem o segundo fator
do fornecedor de identidade, não daqui.

Os administradores encontram também nesta secção o **registo de segurança**:
inícios de sessão e falhas, alterações de palavra-passe e de dois fatores,
alterações de tokens e de contas, com o endereço do cliente. Mantém as últimas
5 000 entradas.

## Notificações push {#push-notifications}

Notificações nativas neste telemóvel ou computador, mesmo com o DumbMonit
fechado: **Ativar neste dispositivo**, depois **Enviar um teste**. Cada conta
gere aqui os seus próprios dispositivos (os visualizadores também); os alertas
que lhes chegam são definidos por um canal **Web Push** em Alertas →
Notificações. O botão **i** resume os requisitos: HTTPS (ou localhost),
instalação no Ecrã Principal no iPhone e iPad, e os domínios dos serviços push
que o servidor tem de alcançar. Guia completo:
[Notificações push (PWA)](push-notifications.md).

## Utilizadores {#users}

Apenas administradores. As contas que podem iniciar sessão, e o seu papel:

| Papel | Lê tudo | Trata alertas (reconhecer, adiar, ignorar, janelas de manutenção, limpar histórico) | Altera a configuração (dispositivos, regras, canais, páginas de estado, definições, cópias de segurança, tokens, contas) |
|---|---|---|---|
| **Viewer** | sim | não | não |
| **Operator** | sim | sim | não |
| **Admin** | sim | sim | sim |

Um viewer vê todas as páginas com todos os controlos que alterariam algo
ocultos. Algumas coisas continuam só para administradores até para leitura:
tokens de API e de agente, emails de subscritores de páginas de estado, e
opções de dispositivos que parecem segredos (uma opção cujo nome contém `key`,
`token`, `password`…, os cabeçalhos extra de uma verificação HTTP, a
palavra-passe de um endereço `https://user:pass@host`), que os não
administradores leem como `***`. Um operator é o papel de piquete: os controlos
de alertas estão lá, os ecrãs de configuração dizem *Operator — config is admin
only*. Só um administrador cria contas ou altera um papel — um operator não
promove ninguém, incluindo a si próprio. As secções *Utilizadores*, *Início de
sessão único* e o registo de segurança continuam só para administradores.
**Adicionar utilizador** pede um nome de utilizador, um nome a apresentar
opcional, um papel e uma palavra-passe; o botão ao lado do campo da
palavra-passe gera uma aleatória. A palavra-passe é mostrada uma só vez, para
entregar.

Cada linha traz o papel, se a conta inicia sessão com palavra-passe ou através
do fornecedor de identidade, se os dois fatores estão ativos, e o último início
de sessão. **Editar** altera o nome a apresentar, o papel e a palavra-passe
(uma reposição termina a sessão desse utilizador em todo o lado e mostra a nova
palavra-passe uma só vez). O interruptor **Ativada** suspende uma conta sem a
apagar; **Reset 2FA** aparece quando a conta tem um segundo fator;
**Eliminar** remove-a de vez. O servidor mantém pelo menos um administrador
ativo, e os controlos refletem essa regra em vez de a reportarem como erro: o
último administrador não pode ser despromovido, desativado nem eliminado, e não
pode desativar nem eliminar a sua própria conta.

## Início de sessão único {#single-sign-on}

Apenas administradores. Início de sessão através de um fornecedor OpenID
Connect (Authelia, Authentik, Keycloak, Pocket ID, Google Workspace…).
Preencha o *nome do fornecedor* (rotula o botão «Continuar com …» no ecrã de
início de sessão), o *URL do emissor* — a descoberta é lida em
`{issuer}/.well-known/openid-configuration` — o *client id*, o *client secret*
(guardado cifrado, nunca mais mostrado) e o *URL público* com que os navegadores
alcançam o DumbMonit. O formulário constrói o URI de redirecionamento a partir
desse URL público: copie-o e registe-o no seu fornecedor.

**Testar descoberta** lê a configuração do fornecedor sem iniciar sessão a
ninguém, lista os endpoints que encontrou, e avisa quando o fornecedor não assina
nem com RS256 nem com ES256 — as duas únicas assinaturas que o DumbMonit aceita.
As definições aqui guardadas têm precedência sobre as variáveis de ambiente
`DUMBMONIT_OIDC_*`, que só se aplicam enquanto nada estiver guardado; **Esquecer
definições guardadas** apaga-as e recorre às variáveis quando existem.

**Avançado.** O resto do formulário fica em *Avançado*: *Scopes* nomeia o que
pedir ao fornecedor, *Groups claim* a claim do token que lista os grupos do
utilizador, e *Criar contas no primeiro início de sessão* decide se uma pessoa
desconhecida recebe uma conta. Está **desativado por predefinição**: só as contas
que já existem podem iniciar sessão até o ativar.

**Papéis.** *Grupos de administradores* lista os grupos cujos membros passam a
administradores; *Grupos de operadores* os grupos cujos outros membros passam a
operadores; todos os restantes são viewers. Deixe ambos vazios para continuar a
gerir os papéis em **Utilizadores**. Com apenas *Grupos de operadores*
preenchido, os grupos nunca tocam no papel de administrador: os administradores
continuam administradores, os outros passam a operadores ou viewers. Os papéis
são reavaliados em cada início de sessão, exceto que o último administrador
ativo nunca é despromovido. As variáveis de ambiente correspondentes são
`DUMBMONIT_OIDC_ADMIN_GROUPS` e `DUMBMONIT_OIDC_OPERATOR_GROUPS`.

**Em que conta um início de sessão aterra.** A identidade é lembrada pelo seu
subject do fornecedor após o primeiro início de sessão. Num primeiro início de
sessão:

- Uma conta existente só é associada quando o fornecedor afirma
  `email_verified: true` e esse email é o nome de utilizador da conta. Um
  `preferred_username` ou um email não verificado nunca associa: na maioria dos
  fornecedores é o utilizador que os escolhe, pelo que `admin` do fornecedor não
  deve tornar-se o *seu* `admin`.
- Um **administrador local que tem palavra-passe nunca é associado
  automaticamente**, mesmo com email verificado: o início de sessão cria antes
  uma conta distinta. Continue a gerir esse administrador com a sua
  palavra-passe, ou crie um administrador SSO separado através dos grupos de
  administradores.
- Caso contrário, com *Criar contas no primeiro início de sessão* ativado, é
  criada uma conta nova (nome de utilizador do fornecedor, com sufixo `-1`,
  `-2`… quando já está ocupado). Com ele desativado, só a associação por email
  verificado acima pode iniciar sessão.

**A criação de contas está desativada a menos que a escolha.** As instalações
novas começam com *Criar contas no primeiro início de sessão* desativado, tal
como qualquer configuração em que nunca foi definido — definições guardadas
antes de a opção existir, ou configurações só por ambiente sem
`DUMBMONIT_OIDC_AUTO_CREATE`. Versões anteriores tratavam «nunca definido» como
ativado; o painel de Início de sessão único diz agora quando esta predefinição
se aplica. Um valor que guardou explicitamente é mantido tal como está.

## Agentes {#agents}

Tokens de registo para o [agente Linux, macOS, FreeBSD e Windows](../devices/agent.md).
Crie um com um nome («Servidor de ficheiros», «Frota doméstica»): o token é
mostrado uma só vez, com os comandos de instalação Linux e Windows prontos a
copiar. Cada comando traz as somas de verificação SHA-256 dos binários do agente
que este servidor fornece, e o instalador recusa um download que não
corresponda; as somas de verificação também estão listadas abaixo dos comandos,
para comparar à mão. A lista mostra o prefixo de cada token, a data de criação,
a última utilização e se foi revogado. **Revogar** pára todos os agentes que
usam esse token no seu próximo envio.

Um token pode registar várias máquinas. Revogá-lo não apaga os dispositivos. A
lista de tokens, a criação e a revogação são para administradores; viewers e
operators veem a secção sem a lista.

## API e assistentes {#api-assistants}

Um só tipo de token para ambos: clientes MCP e a
[API REST](../reference/api.md). Dê um nome a um token, escolha o seu âmbito e
crie-o — é mostrado uma só vez. Um token de **leitura** só pode olhar; um token
de **leitura e escrita** também pode alterar coisas — silenciar um dispositivo,
executar uma sonda, adicionar ou editar dispositivos, regras e canais — mas nunca
contas, definições de início de sessão nem outros tokens.

Em **Ligar**, três separadores contêm os excertos, preenchidos com o token
enquanto ainda está no ecrã: *Claude* (o comando `claude mcp add` e o bloco de
`claude_desktop_config.json`), *ChatGPT* (o URL do servidor MCP e o cabeçalho de
autorização, com um aviso quando esta página não é servida por HTTPS — o ChatGPT
liga a partir dos servidores da OpenAI, pelo que o endereço tem de ser
alcançável a partir da internet) e *Cursor / outro*, a mesma forma para qualquer
cliente que fale Streamable HTTP com um cabeçalho bearer. **Acesso à API**
mostra o mesmo token na API REST como um exemplo de `curl`, e aponta as rotas
que um Prometheus ou um Grafana recolhem com ele. Veja
[Ligar um assistente](assistant.md) e
[Recolher métricas do DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## Cópia de segurança {#backup}

Três coisas, pela ordem em que as encontra.

Um **aviso que não pode falhar**: `/data/secret.key` é o que decifra as
credenciais dos dispositivos, e uma cópia da base de dados sem ele restaura uma
instância que não consegue falar com nada. O texto muda quando o segredo vem de
`DUMBMONIT_SECRET` — não há então ficheiro a copiar, e é o seu gestor de
palavras-passe que o tem de guardar.

**Exportar a configuração** lista o que o pacote conteria, com uma contagem por
secção, e pede depois uma frase-passe duas vezes. O ficheiro que desce contém
todas as credenciais desta instância, cifradas apenas com essa frase-passe:
guarde-o onde guarda palavras-passe. As palavras-passe das contas e os segredos
de 2FA ficam de fora a menos que ative o interruptor.

**Restaurar um pacote** recebe o ficheiro e a sua frase-passe e mostra primeiro
uma **simulação**: uma tabela do que seria criado, atualizado e deixado como
está, secção a secção, com uma nota para tudo o que não consegue colocar. Nada é
escrito até premir *Restore for real*.

**Cópias de segurança locais agendadas** mostra se estão ativadas, para onde
vão, a última execução e se teve êxito, e os ficheiros guardados com o seu
tamanho. **Copiar agora** escreve uma imediatamente — o que convém fazer antes
de uma atualização.

Tudo aqui é só para administradores. Veja
[Cópia de segurança e restauro](../install/backup.md).

## Relatórios {#reports}

Emails periódicos de disponibilidade para a equipa: escolha semanal, diário ou
mensal, o dia, a hora e o fuso horário, os destinatários e o canal de email,
depois **Enviar uma pré-visualização** para ver o resultado. Apenas
administradores. Veja [Relatórios por email](reports.md).

## Aparência {#appearance}

Três mosaicos, cada um a pré-visualizar o seu tema: **Sistema** (segue o seu
dispositivo e muda com ele), **Dia** (o tema «papel de gráfico») e **Noite** (o
tema «radar»). A escolha é guardada no navegador. O interruptor do cabeçalho e
*Alternar tema* da paleta de comandos alternam entre dia e noite.

**Abrir o modo parede** fica no fundo da secção: só o boletim, em ecrã inteiro,
para um monitor na sala. ++esc++ sai dele, e a paleta de comandos abre-o a
partir de qualquer lado — veja [Modo parede](overview.md#wall-mode).

## Acerca de {#about}

A versão do servidor e a saúde dos seus dois armazenamentos, a base de dados
(configuração de dispositivos e histórico de alertas) e o VictoriaMetrics (todas
as medições dos gráficos), conforme reportado por `GET /api/health` e atualizado
a cada 15 segundos. Um armazenamento que deixa de responder mostra um aviso com
o erro e o que verificar. A secção também liga a esta documentação e nomeia a
licença, Apache 2.0.
