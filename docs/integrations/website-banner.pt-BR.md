# Banner de site {#website-banner}

Exiba um banner no seu site quando os serviços estiverem indisponíveis ("2 serviços estão
indisponíveis"), acionado por uma [página de status](../using/status-pages.md) publicada. Quando
tudo está bem, o banner fica oculto, a menos que você peça para ele ser exibido
("Todos os sistemas operacionais").

Três maneiras de adicioná-lo, da mais simples à mais controlada. Arquivos prontos para copiar estão
na pasta `integrations/` do repositório.

## Snippet HTML {#html-snippet}

Cole isto antes de `</body>`, com o endereço do seu servidor e o endereço da página:

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="pt-BR"></script>
```

| Atributo | Valores | Padrão |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues` (apenas quando algo está errado), `always` | `issues` |
| `data-lang` | `en`, `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans` | idioma da página, caso contrário `en` |

O script não possui dependências, desenha dentro de um Shadow DOM para que seus estilos não
o afete, segue a preferência clara ou escura do visitante e pode ser descartado
(até que a aba do navegador seja fechada). Ele lê o JSON público da página, que o
servidor permite que qualquer site busque (`Access-Control-Allow-Origin: *`, sem
cookies). Apenas o documento público da página de status é aberto desta forma; o resto
da API mantém suas regras.

## WordPress {#wordpress}

Copie `integrations/wordpress/dumbmonit-status-banner` para
`wp-content/plugins/`, ative-o e então abra **Configurações > DumbMonit Banner**
e insira a URL da página de status
(`https://monit.example.com/api/public/status/my-page`), a posição e o
modo de exibição. O plugin busca o JSON no servidor, o armazena em cache por um minuto
e escapa tudo que imprime.

O servidor DumbMonit pode estar em um endereço privado (LAN): o plugin apenas
solicita o URL exato que você salvou (verificado em relação ao
padrão `/api/public/status/<address>`, redirecionamentos desabilitados). Se o endereço for
inválido, a página de configurações mostra um erro e mantém o valor anterior. Se a
página de status não puder ser buscada, um aviso é mostrado no admin do WordPress (e
ao salvar), em vez de falhar silenciosamente.

O texto do banner é traduzível (domínio de texto `dumbmonit-status-banner`, modelo
em `languages/`); ele segue o idioma do site WordPress. O JSON de status público
não carrega nenhum idioma, portanto não pode escolher um.

## Inclusão PHP {#php-include}

Para qualquer site PHP, sem estrutura:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

O auxiliar aceita apenas URLs `http` e `https`, usa um tempo limite de 3 segundos, não
segue redirecionamentos, mantém um cache de arquivo no diretório temporário e não imprime
nada se o servidor não puder ser alcançado (a falha é escrita para o
log de erros do PHP). Os textos estão em inglês por padrão; passe `'lang' => 'pt-BR'` ou uma matriz `'strings'`
para alterá-los. Um endereço privado (LAN) funciona. Consulte `integrations/php/example.php`.
