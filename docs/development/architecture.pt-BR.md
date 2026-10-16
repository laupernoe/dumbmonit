# Arquitetura

O DumbMonit é entregue como um único contêiner. Dentro dele há um processo Rust
que serve a interface web e a API, mais um processo filho (VictoriaMetrics) que
o processo Rust inicia e supervisiona. A configuração e o estado ficam no
SQLite.

## Componentes

| Componente | Onde | Função |
|---|---|---|
| Servidor | `crates/server` | O binário Rust único: API HTTP (axum), autenticação, agendador, motor de alertas, canais de notificação, relatórios. |
| Interface web | `web/` | Aplicação de página única SvelteKit (Svelte 5, Tailwind 4, uPlot). Compilada em arquivos estáticos e embutida no binário do servidor com `rust_embed`. |
| Séries temporais | `crates/server/src/tsdb/` | Escritor em lote e proxy de consultas para o VictoriaMetrics. `embedded.rs` inicia e supervisiona o processo filho VictoriaMetrics embutido (dados em `/data/vm`); `DUMBMONIT_VM_URL` aponta para uma instância externa. |
| Configuração e estado | `crates/server/src/db/` | SQLite com migrações numeradas: monitores, credenciais, usuários, regras, estado dos alertas, silêncios, canais. |
| Coletores | `crates/collectors`, `crates/server/src/collectors/` | Uma implementação do trait `Collector` por tipo de monitor ou serviço, registrada em um `Registry`. O agendador só conhece o trait. |
| Tipos compartilhados | `crates/proto` | `Sample`, `Target`, `Credential`, o trait `Collector` e `ProbeError`, e o protocolo do agente. |
| Agente | `crates/agent` | Agente Linux, macOS, FreeBSD e Windows que envia métricas do host; também pode atuar como relay que executa coletores para um site remoto. |
| Alertas | `crates/server/src/alerting/` | Regras, máquina de estados dos alertas, supressão pelo pai, silêncios, linhas de base aprendidas. |
| Notificações | `crates/server/src/notify/` | Canais (chat, push, plantão, e-mail, webhooks), descritos para a interface por `catalog.rs`. |
| Segredos | `crates/server/src/crypto.rs` | AES-256-GCM para credenciais e tokens armazenados no SQLite; a chave vem de `/data/secret.key` ou de `DUMBMONIT_SECRET`. |

## Fluxo de dados

```text
 devices / services ──(SNMP, HTTP APIs, probes)──► collectors ┐
                                                              │ samples
 agents ──(push, per-machine token)──► /api/ingest ───────────┤
 relay agent (collectors run at a remote site) ───────────────┤
                                                              ▼
                                           tsdb writer (batched) ──► VictoriaMetrics
                                                                           │
 browser / scripts / assistants ◄─ /api/* ◄─ query proxy ◄─────────────────┘
                                   │
                                   └─► alerting engine ──► notify channels

 SQLite (config, users, alert state, silences) is read and written by the
 API, the scheduler and the alerting engine.
```

1. O agendador executa cada monitor ativado em seu intervalo através do
   registro de coletores. Agentes e relays enviam suas medições para
   `/api/ingest` com um token por máquina.
2. Os coletores retornam amostras chamadas `dumbmonit_<kind>_*` com um rótulo
   `target` (o id do monitor). O escritor as agrupa em lotes para o
   VictoriaMetrics.
3. O motor de alertas avalia as regras sobre as séries armazenadas. `phase` é o
   estado do motor; `effective_phase` é o que os usuários veem (`suppressed`
   quando um monitor pai está fora do ar). Um monitor inacessível gera
   um alerta; um erro de configuração é exibido, mas não notificado.
4. As notificações saem pelos canais configurados na política de notificação.
5. A interface e qualquer script leem tudo através de `/api/*`; não existe
   metade privada da API.

## Pontos de extensão

- Um novo tipo de monitor é uma implementação de `Collector` mais uma
  chamada a `Registry::register`. Os formulários da interface são gerados a
  partir de `GET /api/collectors`.
- Um novo canal de notificação é um remetente mais uma entrada em
  `notify/catalog.rs`; a interface renderiza o formulário a partir de
  `GET /api/notify/kinds`.
- Um monitor SNMP que só precisa de um perfil não exige Rust: um arquivo
  YAML em `profiles/`.

Veja [Desenvolvimento](../development.md) para as instruções de build e
[Modelo de segurança](security-model.md) para os limites de confiança.
