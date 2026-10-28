# Configurações {#settings}

![Configurações: sete abas, aqui Minha conta com aparência, conta e segurança e notificações push](../assets/screenshots/settings-light.png){ loading=lazy }

As Configurações reúnem o que é administrativo: a própria instância, sua conta, quem pode entrar e como, os agentes, os canais de notificação, o backup da instância e a saúde do servidor. Elas são divididas em sete abas, cada uma com seu próprio endereço. Uma barra em tela larga, uma fileira de chips no celular e uma busca pelos nomes das seções ajudam a encontrar uma seção.

| Aba | Endereço | O que contém |
|---|---|---|
| Geral | `/settings/general` | Nome da instância, idioma e fuso horário, o assistente de configuração ([Interface e assistente de configuração](product-modes.md)) |
| Minha conta | `/settings/account` | Idioma, Conta e segurança (senha, dois fatores, registro de segurança), Notificações push, Contatos de plantão e horário de silêncio ([Plantão](../alerting/oncall.md)) |
| Equipe e acesso | `/settings/team` | Usuários, Equipes, Clientes, Login único, SAML, SCIM, LDAP |
| Monitoramento | `/settings/monitoring` | Agentes, Pacotes de integração, [Pontuação de segurança](security-score.md), Locais |
| Notificações | `/settings/notifications` | Canais, Política de notificações ([Canais de notificação](../notifications.md)) |
| Tela de parede | `/settings/wall` | Modo parede: o que mostra e como abrir ([Modo parede](wall.md)) |
| Sistema | `/settings/system` | Backup, Arquivos de configuração, O próprio DumbMonit, Dados e retenção, Logs, API e assistentes, Sobre |

Cada seção mantém sua âncora: `/settings#agents` redireciona para `/settings/monitoring#agents`. Os links antigos (`/alerts#notifications`) também redirecionam para o novo lugar. As páginas de status e os relatórios não estão mais nas Configurações: eles têm páginas próprias, **Páginas de status** (`/status`) e **Relatórios** (`/reports`); `/settings/status` e `/settings/reports` redirecionam para lá. A música, as notícias e os widgets da parede deixaram de existir: seus links antigos (`#music`, `#news`, `#widgets`) abrem a aba Tela de parede.

## Aparência {#appearance}

O idioma da interface, salvo neste navegador. O console tem uma única aparência
escura, então não há tema para escolher; as páginas de status públicas mantêm
seus temas claro e escuro.

## Tela de parede {#wall}

**Abrir o modo parede**: o painel do console, em tela cheia, para um monitor na
sala. ++esc++ sai, e a paleta de comandos abre de qualquer lugar. A parede não
tem mais música, notícias nem widgets; se quiser, uma cidade à noite pode ficar
atrás do veredito. Veja [Modo parede](wall.md).

## Conta e segurança {#account--security}

Esta é a sua própria conta. A primeira é criada na primeira inicialização,
e mais podem ser adicionadas abaixo. Aqui você muda sua senha: senha
atual, nova senha (pelo menos 12 caracteres; uma frase inteira é mais
segura que uma palavra complicada), confirmação. Mudá-la desconecta toda
outra sessão. Contas que entram através do provedor de identidade não têm
senha aqui. **Sair** termina esta sessão.

O login tem limite de taxa por endereço de cliente e por conta: após cinco
tentativas falhadas, cada tentativa adicional espera mais (30 s, dobrando,
até 5 minutos). Detrás de um proxy reverso, defina
`DUMBMONIT_TRUSTED_PROXIES` para que o limitador veja o endereço real do
cliente em vez do do proxy (veja
[Configuração](../reference/configuration.md)). Sessões duram 30 dias.
Uma senha esquecida é redefinida com `DUMBMONIT_RESET_PASSWORD=1`: veja as
[Perguntas frequentes](../faq.md#i-lost-the-password).

### Autenticação de dois fatores {#two-factor-authentication}

Contas que entram com senha podem adicionar um segundo fator: um código de
seis dígitos baseado em tempo (TOTP) de um app autenticador — Aegis,
FreeOTP, Google Authenticator, 1Password, Bitwarden… **Configurar** pede
sua senha, mostra um código QR para escanear (ou a chave para digitar), e
ativa o segundo fator uma vez que você informa um primeiro código do app.
Oito **códigos de recuperação** são então mostrados uma vez: salve-os,
cada um entra você uma única vez se o celular for perdido. A tela de login
pede o código (ou um código de recuperação) depois da senha; cinco códigos
errados cancelam a tentativa e você recomeça da senha, e o limite de taxa
de login habitual continua contando entre tentativas. Um código é aceito
uma vez: o que você acabou de usar (incluindo o que ativou o segundo
fator) não entra você de novo — espere pelo próximo.

**Desativar** pede a senha de novo. Códigos de recuperação não podem ser
regenerados por conta própria: desative e configure de novo para obter um
conjunto novo. Um administrador pode redefinir o segundo fator de outro
usuário em **Usuários** (*Redefinir 2FA*) quando tanto o celular quanto os
códigos se perderam — isso desconecta o usuário em todo lugar e deixa a
senha intacta. O segredo é armazenado criptografado com o segredo da
instância; códigos são hasheados. Contas de login único recebem seu
segundo fator do provedor de identidade, não aqui.

Administradores também encontram o **log de segurança** nesta seção:
logins e falhas, mudanças de senha e dois fatores, mudanças de token e
conta, com o endereço do cliente. Ele mantém as últimas 5.000 entradas.

## Notificações push {#push-notifications}

Notificações nativas neste celular ou computador, mesmo com o DumbMonit
fechado: **Ativar neste monitor**, depois **Enviar um teste**. Cada
conta gerencia seus próprios monitores aqui (visualizadores também);
quais alertas chegam até eles é definido por um canal **Web Push** em
Configurações → Notificações e relatórios. O botão **i** resume os requisitos: HTTPS (ou
localhost), instalação na Tela de Início no iPhone e iPad, e os domínios
de serviço de push que o servidor precisa alcançar. Guia completo:
[Notificações push (PWA)](push-notifications.md).

## Usuários {#users}

Apenas administradores. As contas que podem entrar, e seu papel:

| Papel | Lê tudo | Lida com alertas (reconhecer, soneca, ignorar, janelas de manutenção, limpar histórico) | Altera a configuração (monitores, regras, canais, páginas de status, configurações, backups, tokens, contas) |
|---|---|---|---|
| **Visualizador** | sim | não | não |
| **Operador** | sim | sim | não |
| **Admin** | sim | sim | sim |

Um visualizador vê toda página com todo controle que mudaria algo
oculto. Um operador é o papel de plantão: os controles de alerta estão
lá, as telas de configuração leem *Operador — configuração é restrita a
administradores*. Só um admin cria contas ou muda um papel — um operador
não pode promover ninguém, incluindo a si mesmo. As seções *Usuários*,
*Login único* e o log de segurança permanecem restritas a administradores.
**Adicionar usuário** pede um nome de usuário, um nome de exibição
opcional, um papel e uma senha; o botão ao lado do campo de senha inventa
uma aleatória. A senha é mostrada uma vez, para ser entregue.

Cada linha carrega o papel, se a conta entra com senha ou através do
provedor de identidade, se o segundo fator está ativo, e o último login.
**Editar** muda o nome de exibição, o papel e a senha (uma redefinição
desconecta esse usuário em todo lugar e mostra a nova senha uma vez). O
botão **Ativado** suspende uma conta sem excluí-la; **Redefinir 2FA**
aparece quando a conta tem um segundo fator; **Excluir** a remove de vez.
O servidor mantém pelo menos um administrador ativo, e os controles
refletem essa regra em vez de reportá-la como um erro: o último
administrador não pode ser rebaixado, desativado ou excluído, e você não
pode desativar ou excluir sua própria conta.

## Login único {#single-sign-on}

Apenas administradores. Login através de um provedor OpenID Connect
(Authelia, Authentik, Keycloak, Pocket ID, Google Workspace…). Preencha o
*nome do provedor* (rotula o botão "Continuar com …" na tela de login), a
*URL do emissor* — a descoberta é lida de
`{issuer}/.well-known/openid-configuration` — o *id do cliente*, o
*segredo do cliente* (armazenado criptografado, nunca mostrado de novo) e
a *URL pública* onde navegadores alcançam o DumbMonit. O formulário
constrói a URI de redirecionamento a partir dessa URL pública: copie-a e a
registre com seu provedor.

**Testar descoberta** lê a configuração do provedor sem conectar ninguém,
lista os endpoints que encontrou, e avisa quando o provedor assina com
nem RS256 nem ES256 — as únicas duas assinaturas que o DumbMonit aceita.
Configurações salvas aqui têm precedência sobre as variáveis de ambiente
`DUMBMONIT_OIDC_*`, que só se aplicam enquanto nada está salvo;
**Esquecer configurações salvas** as limpa e volta às variáveis quando
existem.

**Avançado.** O resto do formulário fica sob *Avançado*: *Escopos* nomeia
o que pedir ao provedor, *Reivindicação de grupos* a reivindicação do
token que lista os grupos do usuário, e *Criar contas no primeiro login*
decide se uma pessoa desconhecida recebe uma conta. Está **desligado por
padrão**: só contas que já existem podem entrar até você ativá-lo.

**Papéis.** *Grupos de admin* lista os grupos cujos membros se tornam
administradores; *Grupos de operador* os grupos cujos outros membros se
tornam operadores; todos os demais são visualizadores. Deixe ambos vazios
para continuar gerenciando papéis sob **Usuários**. Com apenas *Grupos de
operador* preenchido, os grupos nunca tocam o papel de admin: admins
permanecem admins, os outros se tornam operadores ou visualizadores.
Papéis são reavaliados a cada login, exceto que o último admin ativo
nunca é rebaixado. As variáveis de ambiente correspondentes são
`DUMBMONIT_OIDC_ADMIN_GROUPS` e `DUMBMONIT_OIDC_OPERATOR_GROUPS`.

**Em qual conta um login cai.** A identidade é lembrada por seu assunto de
provedor depois do primeiro login. Em um primeiro login:

- Uma conta existente só é vinculada quando o provedor afirma
  `email_verified: true` e esse e-mail é o nome de usuário da conta. Um
  `preferred_username` ou um e-mail não verificado nunca vincula: na
  maioria dos provedores o usuário escolhe esses, então `admin` do
  provedor não deve se tornar *seu* `admin`.
- Um **admin local que tem senha nunca é vinculado automaticamente**,
  mesmo em um e-mail verificado: o login cria uma conta distinta em vez
  disso. Continue gerenciando esse admin com sua senha, ou crie um admin
  SSO separado através dos grupos de admin.
- Caso contrário, com *Criar contas no primeiro login* ativado, uma nova
  conta é criada (nome de usuário do provedor, sufixado `-1`, `-2`…
  quando já em uso). Com desligado, só o vínculo de e-mail verificado
  acima pode entrar.

**A criação de conta está desligada a menos que você a escolha.** Novas
instalações começam com *Criar contas no primeiro login* desligado, e o
mesmo vale para qualquer configuração onde nunca foi definido —
configurações salvas antes da opção existir, ou configurações apenas de
ambiente sem `DUMBMONIT_OIDC_AUTO_CREATE`. Versões anteriores tratavam
"nunca definido" como ligado; o painel de Login único agora diz quando
esse padrão se aplica. Um valor que você salvou explicitamente é mantido
como está.

## Agentes {#agents}

Só o que é comum a todos os [agentes](../devices/agent.md), para os
administradores: o canal criptografado dos agentes (certificados e sua
autoridade, veja [Segurança do agente](../install/agent-security.md)), as
atualizações automáticas dos agentes e os tokens de inscrição já emitidos. Cada
token mostra seu escopo, a data de criação e o último uso; **Revogar** para
todos os agentes que o usam no próximo envio, sem apagar os monitoramentos.

Adicionar uma máquina (token, comando de instalação, primeira conexão, rota por
um relay, kit do Active Directory) acontece no fluxo de adição, **Novo
monitoramento → Agente** (`/targets/new?kind=agent`), não aqui. Os próprios
agentes têm suas páginas.

## Pontuação de segurança {#security-score}

Em **Configurações → Monitoramento**, abaixo dos agentes: uma pontuação de quão bem protegidos estão a instância e os hosts monitorados. Veja [Pontuação de segurança](security-score.md).

## Relatórios {#reports}

Em **Configurações → Notificações e relatórios**, abaixo da política: e-mails
periódicos de disponibilidade para a equipe. Escolha semanal,
diário ou mensal, o dia, hora e fuso horário, os destinatários e o canal
de e-mail, depois **Enviar pré-visualização** para ver o resultado.
Restrito a administradores. Veja [Relatórios por e-mail](reports.md).

## Backup {#backup}

Três coisas, na ordem em que você as encontra.

Um **aviso que você não pode perder**: `/data/secret.key` é o que
descriptografa credenciais de monitor, e uma cópia do banco de dados
sem ele restaura uma instância que não pode falar com nada. O texto muda
quando o segredo vem de `DUMBMONIT_SECRET` em vez disso — não há então
nenhum arquivo para copiar, e é seu gerenciador de senhas que precisa
mantê-lo.

**Exportar a configuração** lista o que o pacote conteria, com uma
contagem por seção, depois pede uma frase secreta duas vezes. O arquivo
que vem contém toda credencial desta instância, criptografada apenas com
essa frase secreta: mantenha-o onde você mantém senhas. Senhas de conta e
segredos de 2FA são deixados de fora a menos que você ative a opção.

**Restaurar um pacote** recebe o arquivo e sua frase secreta e mostra uma
**simulação** primeiro: uma tabela do que seria criado, atualizado e
deixado intacto, seção por seção, com uma nota para qualquer coisa que não
puder colocar. Nada é escrito até você clicar em *Restaurar de verdade*.

**Backups locais programados** mostra se estão ativos, para onde vão, a
última execução e se teve sucesso, e os arquivos mantidos com seu tamanho.
**Fazer backup agora** escreve um imediatamente — a coisa a fazer antes de
uma atualização.

Tudo aqui é restrito a administradores. Veja
[Backup e restauração](../install/backup.md).

## O próprio DumbMonit {#self-monitoring}

O estado do motor agora — agendador, banco SQLite, banco de métricas — com o
espaço livre do volume de dados, o tamanho do banco e, para o VictoriaMetrics
embutido, quantas vezes ele foi reiniciado. Regras integradas já avisam você
quando algo disso dá errado.

Abaixo, o **heartbeat de saída**: uma URL sua (Healthchecks.io, um monitor push
do Uptime Kuma, Better Stack…) que o DumbMonit chama em intervalos regulares,
apenas enquanto está saudável, para que alguém de fora perceba quando este
host fica em silêncio. **Testar** envia uma chamada na hora; **Sinalizar
falhas** chama a URL de falha em vez de ficar em silêncio. A URL é salva
criptografada e nunca mais é exibida, apenas o host. Somente administradores
podem alterá-la. Veja
[Watching DumbMonit itself](../install/self-monitoring.md).

## API e assistentes {#api--assistants}

Um tipo de token para ambos: clientes MCP e a [API REST](../reference/api.md).
Nomeie um token, escolha seu escopo e crie-o — é mostrado uma vez. Um
token **read** só pode olhar; um token **read and write** também pode
alterar coisas — silenciar um monitor, executar uma sondagem,
adicionar ou editar monitores, regras e canais — mas nunca contas,
configurações de login ou outros tokens.

Sob **Conectar**, três abas mantêm os trechos, preenchidos com o token
enquanto ainda está na tela: *Claude* (o comando `claude mcp add` e o
bloco `claude_desktop_config.json`), *ChatGPT* (a URL do servidor MCP e o
cabeçalho de autorização, com um aviso quando esta página não é servida
via HTTPS — o ChatGPT se conecta dos servidores da OpenAI, então o
endereço precisa ser acessível pela internet) e *Cursor / outro*, a mesma
forma para qualquer cliente que fala Streamable HTTP com um cabeçalho
bearer. **Acesso à API** mostra o mesmo token na API REST como um exemplo
`curl`, e aponta para as rotas que um Prometheus ou um Grafana coletam com
ele. Veja [Conectar um assistente](assistant.md) e
[Coletando do DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## Sobre {#about}

Versão do servidor e a saúde de seus dois armazenamentos, o banco de dados
(configuração de monitor e histórico de alertas) e o VictoriaMetrics
(toda medição nos gráficos), como reportado por `GET /api/health` e
atualizado a cada 15 segundos. Um armazenamento que para de responder
mostra um aviso com o erro e o que verificar. A seção também liga para
esta documentação e nomeia a licença, MIT.
