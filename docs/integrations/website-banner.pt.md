# Faixa para o site {#website-banner}

Mostre uma faixa no seu próprio site quando os serviços estão em baixo («2
services are down»), alimentada por uma [página de estado](../using/status-pages.md)
publicada. Quando tudo está bem a faixa fica oculta, a menos que peça para a
mostrar sempre («All systems operational»).

Três formas de a adicionar, da mais simples à mais controlada. Ficheiros
prontos a copiar estão na pasta `integrations/` do repositório.

## Trecho HTML {#html-snippet}

Cole isto antes de `</body>`, com o endereço do seu servidor e o endereço da página:

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| Atributo | Valores | Predefinição |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues` (apenas quando algo está mal), `always` | `issues` |
| `data-lang` | `en`, `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans` | idioma da página, senão `en` |

O script não tem dependências, desenha dentro de um Shadow DOM para que os
seus estilos não o afetem, segue a preferência clara ou escura do visitante e
pode ser dispensado (até o separador do navegador ser fechado). Lê o documento
JSON público da página, que o servidor permite a qualquer site obter
(`Access-Control-Allow-Origin: *`, sem cookies). Só o documento público da
página de estado é aberto desta forma; o resto da API mantém as suas regras.

## WordPress {#wordpress}

Copie `integrations/wordpress/dumbmonit-status-banner` para
`wp-content/plugins/`, ative-o e depois abra **Settings > DumbMonit Banner**
e introduza o URL da página de estado
(`https://monit.example.com/api/public/status/my-page`), a posição e o modo
de exibição. O plugin obtém o JSON no servidor, armazena-o em cache durante
um minuto e escapa tudo o que imprime. O WordPress rejeita pedidos a
endereços privados, pelo que o servidor DumbMonit tem de estar acessível num
endereço público.

## Inclusão PHP {#php-include}

Para qualquer site PHP, sem framework:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

O auxiliar aceita apenas URLs `http` e `https`, usa um tempo limite de 3
segundos, não segue redirecionamentos, mantém uma cache em ficheiro na pasta
temporária e não imprime nada se o servidor não puder ser contactado. Ver
`integrations/php/example.php`.
