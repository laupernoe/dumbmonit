# Plex Media Server {#plex-media-server}

O que o Plex está fazendo e se está atualizado: streams reproduzindo agora,
quantos são transcodificados e se por hardware, a largura de banda que
reservam, streams de fora da casa, bibliotecas sendo varridas, e atualizações.

O DumbMonit lê a própria API do servidor na porta 32400, nunca o plex.tv. Ele
nunca para um stream, nunca varre uma biblioteca e nunca altera uma
configuração.

O Plex não publica o resultado de suas tarefas de manutenção (o "butler" só dá
o cronograma delas) nem o estado de seus backups de banco de dados, então não
há nada a dizer sobre isso.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_plex_`.

| Métrica | O que é | Rótulos |
|---|---|---|
| `version_info` | valor 1 | `version` |
| `update_available` | 1 quando a própria verificação de atualização do Plex encontrou uma versão mais nova; ausente até o Plex ter verificado uma vez | `latest_version` |
| `claimed` | 1 quando o servidor está vinculado a uma conta Plex | |
| `streams`, `streams_paused`, `streams_remote` | reproduções agora; pausadas; de fora da rede local | |
| `transcodes`, `transcodes_video`, `transcodes_hardware` | reproduções recodificadas (vídeo ou áudio); com o vídeo recodificado; com aceleração por hardware | |
| `transcoder_active_video_sessions` | transcodificações de vídeo que o próprio servidor contabiliza | |
| `stream_bandwidth_bits_per_second` | largura de banda reservada por todas as reproduções | |
| `libraries`, `libraries_scanning` | bibliotecas; as que estão sendo varridas agora | |
| `activities` | atividades em segundo plano em execução (varreduras, análise, atualizações) | |
| `scrape_errors` | chamadas que falharam na última sondagem | |

A [regra integrada](../alerting/rules.md#self-hosted-applications) que se
aplica: Plex update available, além de Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca chama o Plex. Ele mostra o estado de atualização e se o servidor está
vinculado, depois streams, transcodificações, transcodificações por hardware,
streams remotos, largura de banda e bibliotecas.

## Dar ao DumbMonit acesso ao servidor Plex {#give-dumbmonit-access-to-the-plex-server}

1. O DumbMonit lê a própria API do servidor na porta 32400, nunca o plex.tv.
   Um servidor vinculado a uma conta Plex exige um token (X-Plex-Token), e o
   Plex não consegue emitir um limitado: é o token do proprietário.

2. Ou evite o token: no Plex, Settings → Network → List of IP addresses and
   networks that are allowed without auth, adicione o endereço do servidor
   DumbMonit, por exemplo assim, e escolha Sem token abaixo.

    ```
    10.0.0.20/32
    ```

3. Ou use o token: no Plex Web, abra qualquer filme → ⋯ → Get Info → View
   XML. O endereço da página que abre termina com X-Plex-Token= seguido do
   token; copie esse valor.

4. No DumbMonit, informe o endereço do servidor, por exemplo "plex.lan"
   (porta 32400). O DumbMonit só lê sessões, bibliotecas, o verificador de
   atualizações e a lista de atividades; ele nunca para um stream e nunca
   altera uma configuração.

!!! warning
    O token é da sua conta Plex: quem o possui pode gerenciar o servidor e
    todas as bibliotecas. Prefira a rede permitida acima quando o DumbMonit
    estiver na sua própria rede.

A rede permitida é também o que faz o Plex responder sem token a partir do
Docker: o contêiner do DumbMonit alcança o Plex pelo seu endereço na rede
Docker, então liste essa rede, ou o endereço do host quando o Plex publica
sua porta no host.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| X-Plex-Token | Enviado no cabeçalho `X-Plex-Token`. |
| Sem token (rede permitida) | Nada é enviado; o Plex precisa listar o endereço do DumbMonit entre as redes permitidas sem autenticação. |

Endereço: um nome de host (`plex.lan`), `host:porta`, ou uma URL completa; um
`/web` final é aceito. Opções: protocolo (HTTP por padrão; os próprios
certificados do Plex são emitidos para nomes `*.plex.direct`, então HTTPS por
endereço IP exige a verificação de certificado desligada), porta (32400),
verificação de certificado e tempo limite da requisição (15 s).

Validado contra o Plex Media Server 1.43.4 (imagem oficial), um servidor não
vinculado respondendo sem token, com uma reprodução transcodificada real. O
caminho do token segue o cabeçalho `X-Plex-Token` documentado pelo Plex e não
foi testado contra um servidor vinculado.
