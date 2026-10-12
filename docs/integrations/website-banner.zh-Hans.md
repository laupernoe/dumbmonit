# 网站横幅 {#website-banner}

当服务宕机时,在你自己的网站上显示一条横幅(“2 项服务不可用”),
横幅由一个已发布的[状态页面](../using/status-pages.md)驱动。当一切
正常时,横幅会保持隐藏,除非你要求始终显示它(“所有系统运行正常”)。

共有三种添加方式,从最简单到最可控。可直接复制使用的文件
位于仓库的 `integrations/` 目录中。

## HTML 代码片段 {#html-snippet}

把下面这段粘贴到 `</body>` 之前,填入你的服务器地址和页面的地址:

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| 属性 | 取值 | 默认值 |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues`(仅在出问题时显示)、`always` | `issues` |
| `data-lang` | `en`, `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans` | 页面语言,否则为 `en` |

该脚本没有任何依赖,会在一个 Shadow DOM 内绘制,因此你的样式不会
影响到它,并会跟随访客的浅色或深色偏好,还可以被关闭(直到浏览器
标签页关闭为止)。它读取页面的公开 JSON 数据,服务器允许任何网站
获取这份数据(`Access-Control-Allow-Origin: *`,不带 cookie)。只有
状态页面的公开文档会以这种方式被打开;API 的其余部分仍遵循
原有的规则。

## WordPress

把 `integrations/wordpress/dumbmonit-status-banner` 复制到
`wp-content/plugins/` 中,启用它,然后打开**设置 > DumbMonit Banner**,
填入状态页面的地址(`https://monit.example.com/api/public/status/my-page`)、
位置和显示模式。该插件会在服务器端获取 JSON 数据,缓存一分钟,
并对打印的所有内容进行转义。WordPress 会拒绝对私有地址发起的请求,
因此 DumbMonit 服务器必须能通过公开地址访问。

## PHP 引入 {#php-include}

适用于任何不使用框架的 PHP 网站:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

该辅助函数只接受 `http` 和 `https` 地址,使用 3 秒超时,不会跟随
重定向,会在临时目录中保留一份文件缓存,并且在无法连接到服务器时
不打印任何内容。参见 `integrations/php/example.php`。
