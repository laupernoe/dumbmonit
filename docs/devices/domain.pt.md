# Expiração de domínio {#domain-expiry}

Um domínio registado, lido do seu registo via RDAP: os dias que faltam até
expirar, uma retenção que o retira do DNS, um período de recuperação, o seu
registador e se a sua delegação está assinada com DNSSEC.

RDAP é o sucessor do WHOIS: os mesmos dados de registo, em JSON, com um
formato padrão entre registos. O DumbMonit encontra o registo da extensão no
ficheiro de arranque RDAP da IANA (`https://data.iana.org/rdap/dns.json`, lido
no máximo uma vez por dia), e depois pergunta-lhe `/domain/example.com`.
Não é necessária nenhuma credencial, e nada além do registo do domínio é
pedido.

Duas falhas merecem atenção além da própria data de expiração. Um domínio em
`clientHold` ou `serverHold` foi retirado do DNS pelo registador ou pelo
registo, geralmente por uma fatura não paga ou um endereço de contacto nunca
verificado: todos os sites e caixas de correio sob ele param de imediato. Um
domínio em `redemptionPeriod` ou `pendingDelete` já expirou e será libertado a
menos que seja restaurado, com um custo.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_domain_`.

| Métrica | O quê | Etiquetas |
|---|---|---|
| `expiry_days` | dias restantes até à expiração, fracionário; negativo depois de expirado | |
| `expiry_timestamp_seconds` | a data de expiração, em segundos Unix | |
| `expiry_published` | 0 quando o registo não publica a data de expiração | |
| `on_hold` | 1 em `clientHold` ou `serverHold` | |
| `redemption` | 1 em `redemptionPeriod`, `pendingDelete` ou `pendingRestore` | |
| `status` | valor 1, uma série por código de estado, em forma EPP (`clientTransferProhibited`) | `status` |
| `registrar_info` | valor 1 | `registrar` |
| `dnssec` | 1 quando a delegação está assinada | |
| `rdap_ok` | 0 quando a última consulta ao registo falhou e é mostrada uma resposta anterior | |
| `rdap_age_seconds` | idade da resposta mostrada | |

O registo é consultado uma vez por intervalo de atualização (12 horas por
predefinição); entretanto, os dias restantes são recalculados a cada
verificação. Quando o registo não responde, a última resposta continua a ser
usada até sete dias, com `rdap_ok` a 0, para que uma falha do registo não
pareça um problema do domínio.

As [regras incorporadas](../alerting/rules.md#reverse-proxies-and-domains) que
se aplicam: Domain expiring (menos de 30 dias), Domain about to expire (menos
de 7 dias ou expirado), Domain on hold, Domain in redemption, mais Device
unreachable.

## A página do monitor {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o registo. Diz a data de expiração e os dias restantes, os
códigos de estado (ou a retenção e recuperação numa frase), o registador e o
DNSSEC, e se a última consulta ao registo falhou.

## Monitorizar o registo de um domínio {#watch-a-domains-registration}

1. Introduza o domínio registado, por exemplo «example.com», não um nome sob ele como www.example.com, que o registo não conhece. Nada para configurar e nenhuma credencial: o RDAP, sucessor do WHOIS, é público.

2. O DumbMonit encontra o registo da extensão no ficheiro de arranque RDAP da IANA, e depois pergunta-lhe a data de expiração, os códigos de estado e o registador, duas vezes por dia por predefinição. Os registos limitam as consultas RDAP, e a data de expiração muda uma vez por ano: um intervalo de atualização mais curto não ganha nada.

3. Alguns registos executam RDAP sem estarem listados pela IANA, os de .ch e .li por exemplo: defina RDAP server para o seu endereço. Uma extensão cujo registo não executa nenhum serviço RDAP público não pode ser monitorizada.

    ```
    https://rdap.nic.ch/
    ```

4. Alguns registos não publicam a data de expiração (.ch é um deles): o DumbMonit diz-o então, e continua a monitorizar os códigos de estado.

!!! warning
    Um domínio em clientHold ou serverHold é retirado do DNS: todos os sites e caixas de correio sob ele param, geralmente por uma fatura não paga ou um endereço de contacto não verificado. O DumbMonit avisa 30 dias antes da expiração, agrava 7 dias antes, e alerta de imediato numa retenção ou período de recuperação.

Um domínio que o registo não conhece (um subdomínio, um erro de escrita, ou um
domínio que foi eliminado) é reportado como um erro de configuração, não como
uma indisponibilidade. Nomes com acentos são aceites e pedidos na sua forma
ASCII (`xn--…`).

## Credenciais {#credentials}

Nenhuma: o RDAP é público.

Endereço: o domínio registado (`example.com`); um URL colado é reduzido ao seu
nome de anfitrião. Opções: servidor RDAP (vazio: encontrado via IANA),
intervalo de atualização (12 horas, de 1 a 168) e tempo limite do pedido
(10 s). O intervalo de verificação do próprio monitor pode ficar na sua
predefinição: só recalcula os dias restantes entre duas consultas ao registo.
