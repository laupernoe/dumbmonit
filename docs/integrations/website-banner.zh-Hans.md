# 网站横幅 {#website-banner}

当服务关闭时在你自己的网站上显示横幅（"2个服务关闭"），由发布的[状态页面](../using/status-pages.md)驱动。当一切都好时横幅保持隐藏，除非你要求显示它（"所有系统正常"）。

三种添加方式，从最简单到最受控。现成副本文件在仓库的 `integrations/` 文件夹中。

## HTML片段 {#html-snippet}

在 `</body>` 前粘贴这个，用你的服务器地址和页面的地址：

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| 属性 | 值 | 默认 |
|---|---|---|
| `data-position` | `top`、`bottom` | `top` |
| `data-show` | `issues`（仅当错误时）、`always` | `issues` |
| `data-lang` | `en`、`fr`、`de`、`es`、`it`、`pt`、`pt-BR`、`ru`、`zh-Hans` | 页面语言，否则`en` |

脚本没有依赖，在Shadow DOM中绘制所以你的样式不接触它，跟随访问者的明亮或深暗偏好，可被解除（直到浏览器标签关闭）。它读页面的公共JSON，服务器允许任何网站获取（`Access-Control-Allow-Origin: *`，无cookies）。仅状态页面的公共文档以这方式打开；API的其余保留其规则。

## WordPress {#wordpress}

复制 `integrations/wordpress/dumbmonit-status-banner` 进入 `wp-content/plugins/`，激活它，然后打开**设置>DumbMonit横幅** 并输入状态页面URL (`https://monit.example.com/api/public/status/my-page`)、位置和显示模式。插件在服务器上获取JSON，为一分钟缓存它，并逃避它打印的一切。

DumbMonit服务器可能在私有（LAN）地址上：插件仅请求你保存的准确URL（针对 `/api/public/status/<address>` 模式检查，重定向禁用）。如果地址无效，设置页面显示错误并保留之前的值。如果状态页面无法获取，在WordPress管理中显示警告（以及保存时），而不是默默失败。

横幅文本可翻译（文本域 `dumbmonit-status-banner`，模板在 `languages/`）；它跟随WordPress网站语言。公共状态JSON没有语言，所以它无法选一个。

## PHP包含 {#php-include}

对于任何PHP网站，没有框架：

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

助手接受 `http` 和 `https` URLs仅，使用3秒超时，不跟随重定向，在临时目录中保留文件缓存，如果无法到达服务器打印无。文本默认用英文；通过 `'lang' => 'fr'` 或 `'strings'` 数组改变它们。私有（LAN）地址有效。见 `integrations/php/example.php`。
