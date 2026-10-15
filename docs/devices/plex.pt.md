# Plex Media Server {#plex-media-server}

O que o Plex está a fazer e se está atualizado: streams a reproduzir agora,
quantos são transcodificados e se em hardware, a largura de banda que reservam,
streams a partir de fora de casa, bibliotecas a ser analisadas, e atualizações.

O DumbMonit lê a própria API do servidor na porta 32400, nunca o plex.tv. Nunca
para um stream, nunca analisa uma biblioteca e nunca altera uma definição.

O Plex não publica o resultado das suas tarefas de manutenção (o «butler» só
dá o seu calendário) nem o estado das suas cópias de segurança da base de
dados, por isso não há nada a dizer sobre isso.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_plex_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `update_available` | 1 quando a própria verificação de atualização do Plex encontrou uma versão mais recente; ausente até o Plex ter verificado uma vez | `latest_version` |
| `claimed` | 1 quando o servidor está associado a uma conta Plex | |
| `streams`, `streams_paused`, `streams_remote` | reproduções agora; em pausa; de fora da rede local | |
| `transcodes`, `transcodes_video`, `transcodes_hardware` | reproduções recodificadas (vídeo ou áudio); com o vídeo recodificado; com aceleração por hardware | |
| `transcoder_active_video_sessions` | transcodificações de vídeo que o próprio servidor conta | |
| `stream_bandwidth_bits_per_second` | largura de banda reservada por todas as reproduções | |
| `libraries`, `libraries_scanning` | bibliotecas; as que estão a ser analisadas agora | |
| `activities` | atividades em segundo plano em execução (análises, análise de conteúdo, atualizações) | |
| `scrape_errors` | chamadas que falharam na última verificação | |

A [regra incorporada](../alerting/rules.md#self-hosted-applications) que se
aplica: Plex update available, mais Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca chama o Plex. Dá o estado da atualização e se o servidor está associado,
depois streams, transcodificações, transcodificações por hardware, streams
remotos, largura de banda e bibliotecas.

## Dar acesso ao servidor Plex ao DumbMonit {#give-dumbmonit-access-to-the-plex-server}

1. O DumbMonit lê a própria API do servidor na porta 32400, nunca o plex.tv. Um servidor associado a uma conta Plex exige um token (X-Plex-Token), e o Plex não consegue emitir um limitado: é o token do proprietário.

2. Ou evite o token: no Plex, Settings → Network → List of IP addresses and networks that are allowed without auth, adicione o endereço do servidor DumbMonit, por exemplo assim, e escolha No token abaixo.

    ```
    10.0.0.20/32
    ```

3. Ou use o token: no Plex Web, abra qualquer filme → ⋯ → Get Info → View XML. O endereço da página que abre termina com X-Plex-Token= seguido do token; copie esse valor.

4. No DumbMonit, introduza o endereço do servidor, por exemplo «plex.lan» (porta 32400). O DumbMonit só lê sessões, bibliotecas, o atualizador e a lista de atividades; nunca para um stream e nunca altera uma definição.

!!! warning
    O token é o da sua conta Plex: quem o detém pode gerir o servidor e todas as bibliotecas. Prefira a rede permitida acima quando o DumbMonit está na sua própria rede.

A rede permitida é também o que faz o Plex responder sem token a partir do
Docker: o contentor do DumbMonit alcança o Plex a partir do endereço da sua
rede Docker, por isso liste essa rede, ou o endereço do anfitrião quando o
Plex publica a sua porta no anfitrião.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| X-Plex-Token | Enviado no cabeçalho `X-Plex-Token`. |
| No token (allowed network) | Nada é enviado; o Plex tem de listar o endereço do DumbMonit entre as redes permitidas sem autenticação. |

Endereço: um nome de anfitrião (`plex.lan`), `anfitrião:porta`, ou um URL
completo; um `/web` final é aceite. Opções: protocolo (HTTP por predefinição;
os próprios certificados do Plex são emitidos para nomes `*.plex.direct`, por
isso HTTPS por endereço IP precisa da verificação de certificado desligada),
porta (32400), verificação de certificado e tempo limite do pedido (15 s).

Validado com o Plex Media Server 1.43.4 (imagem oficial), um servidor não
associado a responder sem token, com uma reprodução real transcodificada. O
caminho do token segue o cabeçalho `X-Plex-Token` documentado pelo Plex e não
foi testado num servidor associado.
