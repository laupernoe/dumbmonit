<?php require __DIR__ . '/banner.php'; ?>
<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>My website</title></head>
<body>
<?php
dumbmonit_banner('https://monit.example.com/api/public/status/my-page', [
    'position' => 'top',    // top | bottom
    'show' => 'issues',     // issues | always
    'ttl' => 60,            // seconds the JSON is cached on disk
]);
?>
<h1>My website</h1>
</body>
</html>
