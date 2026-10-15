# AdGuard Home {#adguard-home}

O próprio filtro DNS: se a proteção está ativa, se seu servidor DNS está
rodando, o que respondeu e bloqueou, se suas listas de filtro ainda
atualizam, se seus servidores upstream respondem, e se uma versão mais
nova existe.

Um filtro DNS quebrado não parece quebrado. Páginas continuam abrindo, com
seus anúncios. As falhas habituais são silenciosas: proteção pausada "por
uma hora" a partir do app do celular e nunca reativada, listas de filtro que
pararam de baixar há semanas, um servidor DNS-over-HTTPS upstream que não
responde mais fazendo toda busca esperar pelo próximo.

O DumbMonit lê a API REST do AdGuard Home (`/control`) com autenticação
básica HTTP. Testado contra o AdGuard Home v0.107.79 (e a verificação de
versão v0.107.78).

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_adguard_`. Números de consulta e
bloqueio são **somas sobre a janela de estatísticas do AdGuard Home** (24
horas a menos que alterado em Settings → General settings), não
contadores: eles sobem e desem conforme a janela deslizam, e são
armazenados como estão.

| Métrica | O que é | Rótulos |
|---|---|---|
| `running` | 1 quando o servidor DNS roda | |
| `protection_enabled` | 1 quando a proteção está ativa | |
| `protection_paused_seconds` | tempo restante antes de uma proteção pausada voltar a ativar, 0 sem pausa | |
| `version_info` | valor 1 | `version` |
| `dns_queries` | consultas na janela de estatísticas | |
| `blocked_filtering` | consultas bloqueadas pelas listas de filtro e regras personalizadas | |
| `replaced_safebrowsing`, `replaced_parental`, `replaced_safesearch` | respostas substituídas por Browsing security, Parental control, Safe search | |
| `blocked_percent` | parcela de consultas bloqueadas (filtros, Browsing security, Parental control); ausente sem nenhuma consulta | |
| `avg_processing_seconds` | tempo médio para responder | |
| `stats_window_seconds`, `stats_enabled` | a janela de estatísticas e se estatísticas são mantidas (0.107.30 e posteriores) | |
| `upstream_responses`, `upstream_avg_seconds` | respostas e tempo médio de resposta por upstream na janela, como o painel Top upstreams os mostra (no máximo 8) | `upstream` |
| `upstream_up` | 1 quando o próprio teste do AdGuard Home desse upstream passa; 0 quando falha ou não dá resposta dentro de 4 segundos (no máximo 8). O rótulo é o upstream como o AdGuard Home o escreve em suas estatísticas (`https://dns10.quad9.net:443/dns-query`, `192.0.2.1:53`); para um upstream que permaneceu silencioso, a linha como configurada. | `upstream` |
| `filtering_enabled` | 1 quando a filtragem está ativa em Settings → General settings | |
| `filters`, `filters_enabled` | listas de bloqueio configuradas, e ativadas | |
| `filter_rules` | regras carregadas das listas ativadas | |
| `filter_rules_count`, `filter_update_age_seconds` | por lista ativada (no máximo 32) | `filter` |
| `filter_oldest_update_age_seconds` | idade da atualização mais antiga entre as listas ativadas | |
| `filters_never_updated` | listas ativadas que nunca foram baixadas | |
| `filter_update_interval_hours` | com que frequência as listas são atualizadas; 0 desliga as atualizações | |
| `update_check_enabled` | 0 quando o AdGuard Home roda com sua verificação de versão desligada | |
| `update_available` | 1 quando a última verificação de versão anunciou uma versão mais nova; ausente quando a verificação está desligada | `latest` |
| `scrape_errors` | chamadas secundárias que falharam na última verificação | |

A verificação de versão está desligada na imagem Docker oficial e na
maioria dos pacotes de distribuição, que atualizam o AdGuard Home eles
mesmos: ali, `update_available` nunca é escrito e sua regra permanece
quieta.

Estatísticas de upstream só contam respostas: um upstream que para de
responder não deixa rastro nelas. É por isso que o DumbMonit pede ao
AdGuard Home para testar cada upstream a cada verificação, o mesmo teste do
botão Test upstreams em Settings → DNS settings: uma consulta DNS do
AdGuard Home para esse upstream. Um upstream morto deixaria o AdGuard Home
esperando o dobro de seu tempo limite de upstream (20 segundos por padrão);
o DumbMonit para de esperar depois de 4 segundos e o reporta como fora do
ar. Desmarque a opção Testar servidores upstream para desligar isso.

As [regras integradas](../alerting/rules.md#adguard-home) que se aplicam:
AdGuard Home protection off, AdGuard Home DNS not running, AdGuard Home
upstream failing, AdGuard Home filter lists stale, AdGuard Home update
available, além de Monitor unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca se conecta ao AdGuard Home. Ele diz se a proteção e o servidor DNS
estão ativos (e quando uma pausa termina), depois as consultas, bloqueios e
tempo de resposta na janela de estatísticas, os servidores upstream com seu
resultado de teste e tempo de resposta, e as listas de filtro com suas
regras e última atualização.

## Adicionar um usuário para o DumbMonit no AdGuard Home {#add-a-user-for-dumbmonit-to-adguard-home}

1. O AdGuard Home não tem conta somente leitura: todo usuário listado em
   AdGuardHome.yaml pode alterar toda configuração. Dê ao DumbMonit um
   usuário próprio, para que possa ser removido sem tocar no seu. Primeiro
   faça um hash bcrypt de uma senha longa e aleatória (htpasswd vem com o
   pacote apache2-utils ou httpd-tools):

    ```
    htpasswd -B -C 10 -n -b dumbmonit 'LONG-RANDOM-PASSWORD'
    ```

2. Pare o AdGuard Home, abra AdGuardHome.yaml (ao lado do binário
   AdGuardHome, ou na pasta conf do volume Docker) e adicione esta linha
   sob "users:", na mesma indentação da linha "- name:" já existente.
   Substitua HASH pelo que o htpasswd imprimiu depois de "dumbmonit:".
   Inicie o AdGuard Home de novo.

    ```
    - { name: dumbmonit, password: "HASH" }
    ```

    A seção então fica assim:

    ```yaml
    users:
      - { name: dumbmonit, password: "$2y$10$…" }
      - name: yourname
        password: $2a$10$…
    ```

3. Verifique a partir do host DumbMonit que o usuário é aceito:

    ```
    curl -u dumbmonit http://adguard.lan/control/status
    ```

4. No DumbMonit, informe o endereço da interface web do AdGuard Home, por
   exemplo "adguard.lan" (porta 80) ou "http://adguard.lan:3000", depois o
   usuário dumbmonit e sua senha.

5. O DumbMonit só lê o status, as estatísticas, as listas de filtro e a
   última verificação de versão. Ele nunca lê o log de consultas. A cada
   verificação ele pede ao AdGuard Home para testar cada servidor DNS
   upstream, como o botão Test upstreams faz; desmarque Testar servidores
   upstream abaixo para parar isso.

!!! warning
    O usuário dumbmonit pode alterar toda configuração, como qualquer
    usuário do AdGuard Home: mantenha sua senha só para o DumbMonit. Ela
    viaja em cada requisição, então em uma rede não confiável ative HTTPS
    em Settings → Encryption settings. Após cinco senhas erradas, o
    AdGuard Home recusa o endereço por 15 minutos.

O AdGuard Home precisa estar parado enquanto você edita o arquivo: ele
reescreve AdGuardHome.yaml sempre que uma configuração muda, e descartaria
uma linha adicionada enquanto roda.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de usuário / senha | O usuário dedicado adicionado ao AdGuardHome.yaml, enviado como autenticação básica HTTP. |
| Sem autenticação | Apenas para um AdGuard Home sem usuário alguma em AdGuardHome.yaml. |

Endereço: um nome de host ou IP (`adguard.lan`), `host:porta`, ou uma URL
completa com um prefixo de caminho detrás de um proxy reverso
(`https://dns.lan/adguard`); um `/control` final é ignorado. Opções:
protocolo (HTTP por padrão), porta (80), verificação de certificado, tempo
limite da requisição (5 s) e o teste de upstream (ativado).

## Chamadas {#calls}

| Chamada | Por quê |
|---|---|
| `GET /control/status` | em execução, proteção, versão: a única chamada cuja falha falha a verificação |
| `GET /control/stats`, `GET /control/stats/config` | consultas, bloqueios, respostas de upstream, janela de estatísticas |
| `GET /control/filtering/status` | listas de filtro |
| `POST /control/version.json` com `{"recheck_now": false}` | o resultado da última verificação de versão, como a interface web a lê em cada carregamento de página; ela nunca força uma nova verificação |
| `GET /control/dns_info`, depois `POST /control/test_upstream_dns` uma vez por upstream | os upstreams e servidores bootstrap configurados, depois o teste de upstream; não altera nenhuma configuração |
