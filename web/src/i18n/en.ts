export default {
  // navigation
  "nav.basic": "Basic",
  "nav.server": "Server",
  "nav.location": "Location",
  "nav.upstream": "Upstream",
  "nav.proxyPlugin": "ProxyPlugin",
  // basic info
  "basic.title": "Modify the basic configurations",
  "basic.description":
    "The basic configuration of pingap mainly includes various configurations such as logs, graceful restart, threads, etc.",
  "basic.name": "Name",
  "basic.pidFile": "Pid File",
  "basic.upgradeSock": "Upgrade Sock",
  "basic.threads": "Threads",
  "basic.workStealing": "Work Stealing",
  "basic.user": "User",
  "basic.group": "Group",
  "basic.gracePeriod": "Grace Period",
  "basic.gracefulShutdownTimeout": "Graceful Shutdown Timeout",
  "basic.logLevel": "Log Level",
  "basic.upstreamKeepalivePoolSize": "Upstream Keepalive Pool Size",
  "basic.webhookType": "Webhook Type",
  "basic.webhook": "Webhook Url",
  "basic.sentry": "Sentry",
  "basic.pyroscope": "Pyroscope",
  "basic.errorTemplate": "Error Template",
  // server info
  "server.title": "Modify server configuration",
  "server.description": "Change the server configuration",
  "server.addr": "Listen Address",
  "server.locations": "Locations",
  "server.threads": "Threads",
  "server.accessLog": "Access Log",
  "server.tlsCert": "Tls Cert Pem",
  "server.tlsKey": "Tls Key Pem",
  "server.letsEncrypt": "Let's encrypt domain list",
  "server.enabledH2": "Enable Http2",
  "server.remark": "Remark",
  // location info
  "location.title": "Modify location configuration",
  "location.description": "Change the location configuration",
  "location.host": "Host",
  "location.path": "Path",
  "location.upstream": "Upstream",
  "location.weight": "Weight",
  "location.headers": "Headers",
  "location.proxyHeaders": "Proxy Headers",
  "location.rewrite": "Rewrite",
  "location.proxyPlugins": "Proxy Plugins",
  "location.remark": "Remark",
  // upstream info
  "upstream.title": "Modify upstream configuration",
  "upstream.description": "Change the upstream configuration",
  "upstream.addrs": "Upstream Addrs",
  "upstream.algo": "Load balancer algorithm",
  "upstream.healthCheck": "Health Check",
  "upstream.connectionTimeout": "Connection Timeout",
  "upstream.totalConnectionTimeout": "Total Connection Timeout",
  "upstream.readTimeout": "Read Timeout",
  "upstream.writeTimeout": "Write Timeout",
  "upstream.idleTimeout": "Idle Timeout",
  "upstream.alpn": "Alpn",
  "upstream.sni": "Sni",
  "upstream.verifyCert": "Verify Cert",
  "upstream.ipv4Only": "Ipv4 Only",
  "upstream.remark": "Remark",
  // proxy plugin info
  "proxyPlugin.title": "Modify proxy plugin configuration",
  "proxyPlugin.description": "Change the proxy plugin configuration",
  "proxyPlugin.step": "Proxy Exec Step",
  "proxyPlugin.category": "Proxy Plugin Category",
  "proxyPlugin.config": "Proxy Plugin Config",
  "proxyPlugin.remark": "Remark",
  // form
  "form.name": "Name",
  "form.removing": "Removing",
  "form.remove": "Remove",
  "form.submitting": "Submitting",
  "form.submit": "Submit",
  "form.success": "Update success!",
  "form.confirmRemove": "Remove config?",
  "form.removeDescript":
    "Please confirm whether you want to delete the configuration, and it can not be restored after delete it.",
  "form.confirm": "Confirm",
  "form.nameRequired": "Name is required",
  "form.nameExists": "Name is exitst",
  "form.sortPlugin": "Sort proxy plugins",
  "form.selectPlugin": "Select proxy plugin",
  "form.addr": "Addr",
  "form.weight": "Weight",
  "from.addrs": "Add addr",
  "form.header": "Add response header",
  "form.headerName": "Header Name",
  "form.headerValue": "Header Value",
  "form.addProxyHeader": "Add proxy request header",
  "form.proxyHeaderName": "Header Name",
  "form.proxyHeaderValue": "Header Value",
};
