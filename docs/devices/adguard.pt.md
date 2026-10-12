# AdGuard Home {#adguard-home}

O próprio filtro DNS: se a proteção está ativa, se o seu servidor DNS
corre, o que respondeu e bloqueou, se as suas listas de filtros ainda se
atualizam, se os seus servidores a montante respondem, e se uma versão
mais recente está disponível.

Um filtro DNS avariado não parece avariado. As páginas continuam a abrir,
com os seus anúncios. As falhas habituais são silenciosas: proteção pausada
«por uma hora» a partir da app do telefone e nunca mais reativada, listas
de filtros que deixaram de descarregar há semanas, um servidor DNS-over-HTTPS
a montante que já não responde pelo que toda procura espera pelo seguinte.

O DumbMonit lê a API REST do AdGuard Home (`/control`) com autenticação
HTTP básica. Testado contra o AdGuard Home v0.107.79 (e a verificação de
versão v0.107.78).

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_adguard_`. Os valores de
consultas e bloqueios são **somas sobre a janela de estatísticas do AdGuard
Home** (24 horas a menos que alterado em Settings → General settings), não
contadores: sobem e desçem conforme a janela deslizar, e são guardados como
estão.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `running` | 1 quando o servidor DNS corre | |
| `protection_enabled` | 1 quando a proteção está ativa | |
| `protection_paused_seconds` | tempo restante antes de uma proteção pausada voltar a ligar, 0 sem pausa | |
| `version_info` | valor 1 | `version` |
| `dns_queries` | consultas na janela de estatísticas | |
| `blocked_filtering` | consultas bloqueadas pelas listas de filtros e regras personalizadas | |
| `replaced_safebrowsing`, `replaced_parental`, `replaced_safesearch` | respostas substituídas por Browsing security, Parental control, Safe search | |
| `blocked_percent` | fração de consultas bloqueadas (filtros, Browsing security, Parental control); ausente sem nenhuma consulta | |
| `avg_processing_seconds` | tempo médio para responder | |
| `stats_window_seconds`, `stats_enabled` | a janela de estatísticas e se as estatísticas são mantidas (0.107.30 e posterior) | |
| `upstream_responses`, `upstream_avg_seconds` | respostas e tempo médio de resposta por servidor a montante na janela, como o Top upstreams do painel os mostra (no máximo 8) | `upstream` |
| `upstream_up` | 1 quando o próprio teste do AdGuard Home desse servidor a montante passa; 0 quando falha ou não dá nenhuma resposta dentro de 4 segundos (no máximo 8). A etiqueta é o servidor a montante como o AdGuard Home o escreve nas suas estatísticas (`https://dns10.quad9.net:443/dns-query`, `192.0.2.1:53`); para um servidor a montante que ficou silencioso, a linha como configurada. | `upstream` |
| `filtering_enabled` | 1 quando a filtragem está ativa em Settings → General settings | |
| `filters`, `filters_enabled` | listas de bloqueio configuradas, e ativadas | |
| `filter_rules` | regras carregadas das listas ativadas | |
| `filter_rules_count`, `filter_update_age_seconds` | por lista ativada (no máximo 32) | `filter` |
| `filter_oldest_update_age_seconds` | idade da atualização mais antiga entre as listas ativadas | |
| `filters_never_updated` | listas ativadas que nunca foram descarregadas | |
| `filter_update_interval_hours` | com que frequência as listas são atualizadas; 0 desliga as atualizações | |
| `update_check_enabled` | 0 quando o AdGuard Home corre com a sua verificação de versão desligada | |
| `update_available` | 1 quando a última verificação de versão anunciou uma versão mais recente; ausente quando a verificação está desligada | `latest` |
| `scrape_errors` | chamadas secundárias que falharam na última verificação | |

A verificação de versão está desligada na imagem Docker oficial e na
maioria dos pacotes de distribuição, que atualizam o AdGuard Home eles
próprios: aí, `update_available` nunca é escrita e a sua regra fica
silenciosa.

As estatísticas de servidores a montante só contam respostas: um servidor
a montante que deixa de responder não deixa nenhum rasto nelas. É por isso
que o DumbMonit pede ao AdGuard Home para testar cada servidor a montante
em cada verificação, o mesmo teste do botão Test upstreams em Settings →
DNS settings: uma consulta DNS do AdGuard Home para esse servidor a
montante. Um servidor a montante morto deixaria o AdGuard Home à espera
durante o dobro do seu tempo limite a montante (20 segundos por
predefinição); o DumbMonit para de esperar depois de 4 segundos e reporta-o
como em baixo. Desmarque a opção Test upstream servers para desligar isto.

As [regras incorporadas](../alerting/rules.md#adguard-home) que se aplicam:
AdGuard Home protection off, AdGuard Home DNS not running, AdGuard Home
upstream failing, AdGuard Home filter lists stale, AdGuard Home update
available, mais Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca liga ao AdGuard Home. Mostra se a proteção e o servidor DNS estão
ativos (e quando termina uma pausa), depois as consultas, bloqueios e tempo
de resposta ao longo da janela de estatísticas, os servidores a montante
com o seu resultado de teste e tempo de resposta, e as listas de filtros
com as suas regras e última atualização.

## Adicionar um utilizador para o DumbMonit no AdGuard Home {#add-a-user-for-dumbmonit-to-adguard-home}

1. O AdGuard Home não tem conta só de leitura: todo utilizador listado em AdGuardHome.yaml pode alterar toda definição. Dê ao DumbMonit um utilizador próprio, para que possa ser removido sem tocar no seu. Primeiro crie um hash bcrypt de uma senha longa e aleatória (htpasswd vem com o pacote apache2-utils ou httpd-tools):

    ```
    htpasswd -B -C 10 -n -b dumbmonit 'LONG-RANDOM-PASSWORD'
    ```

2. Pare o AdGuard Home, abra AdGuardHome.yaml (junto ao binário AdGuardHome, ou na pasta conf do volume Docker) e adicione esta linha sob «users:», à mesma indentação da linha «- name:» já existente. Substitua HASH pelo que o htpasswd imprimiu depois de «dumbmonit:». Inicie o AdGuard Home de novo.

    ```
    - { name: dumbmonit, password: "HASH" }
    ```

    A secção lê-se então:

    ```yaml
    users:
      - { name: dumbmonit, password: "$2y$10$…" }
      - name: yourname
        password: $2a$10$…
    ```

3. Verifique a partir do anfitrião do DumbMonit que o utilizador é aceite:

    ```
    curl -u dumbmonit http://adguard.lan/control/status
    ```

4. No DumbMonit, introduza o endereço da interface web do AdGuard Home, por exemplo «adguard.lan» (porta 80) ou «http://adguard.lan:3000», depois o utilizador dumbmonit e a sua senha.

5. O DumbMonit só lê o estado, as estatísticas, as listas de filtros e a última verificação de versão. Nunca lê o registo de consultas. A cada verificação pede ao AdGuard Home para testar cada servidor DNS a montante, como o botão Test upstreams faz; desmarque Test upstream servers abaixo para parar isso.

!!! warning
    O utilizador dumbmonit pode alterar toda definição, como qualquer utilizador do AdGuard Home: mantenha a sua senha só para o DumbMonit. Viaja com cada pedido, por isso numa rede não confiável ative o HTTPS em Settings → Encryption settings. Depois de cinco senhas erradas, o AdGuard Home recusa o endereço durante 15 minutos.

O AdGuard Home tem de estar parado enquanto edita o ficheiro: reescreve o
AdGuardHome.yaml sempre que uma definição muda, e descartaria uma linha
adicionada enquanto corre.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador / senha | O utilizador dedicado adicionado ao AdGuardHome.yaml, enviado como autenticação HTTP básica. |
| Sem autenticação | Só para um AdGuard Home sem nenhum utilizador no AdGuardHome.yaml. |

Endereço: um nome de anfitrião ou IP (`adguard.lan`), `anfitrião:porta`, ou
um URL completo com um prefixo de caminho atrás de um proxy inverso
(`https://dns.lan/adguard`); um `/control` final é ignorado. Opções:
protocolo (HTTP por predefinição), porta (80), verificação de certificado,
tempo limite do pedido (5 s) e o teste de servidores a montante (ativado).

## Chamadas {#calls}

| Chamada | Porquê |
|---|---|
| `GET /control/status` | em execução, proteção, versão: a única chamada cuja falha faz a verificação falhar |
| `GET /control/stats`, `GET /control/stats/config` | consultas, bloqueios, respostas a montante, janela de estatísticas |
| `GET /control/filtering/status` | listas de filtros |
| `POST /control/version.json` com `{"recheck_now": false}` | o resultado da última verificação de versão, como a interface web a lê em cada carregamento de página; nunca força uma nova verificação |
| `GET /control/dns_info`, depois `POST /control/test_upstream_dns` uma vez por servidor a montante | os servidores a montante e de bootstrap configurados, depois o teste a montante; não altera nenhuma definição |
