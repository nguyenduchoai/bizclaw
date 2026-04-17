// BizClaw PWA Service Worker
// Version: 1.1.7

const CACHE_NAME = 'bizclaw-v1';
const STATIC_CACHE_NAME = 'bizclaw-static-v1';
const DYNAMIC_CACHE_NAME = 'bizclaw-dynamic-v1';

// Static assets to cache immediately
const STATIC_ASSETS = [
  '/',
  '/index.html',
  '/new',
  '/app.js',
  '/app-new.js',
  '/shared.js',
  '/styles.css',
  '/dashboard-new.css',
  '/sme.css',
  '/vendor/preact.mjs',
  '/vendor/hooks.mjs',
  '/vendor/htm.mjs',
  '/i18n/vi.js',
  '/i18n/en.js',
  '/manifest.json'
];

// API endpoints to cache with stale-while-revalidate
const API_CACHE_RULES = [
  { pattern: /\/api\/v1\/config/, strategy: 'staleWhileRevalidate', maxAge: 5 * 60 * 1000 },
  { pattern: /\/api\/v1\/health/, strategy: 'networkFirst', maxAge: 30 * 1000 },
  { pattern: /\/api\/v1\/providers/, strategy: 'staleWhileRevalidate', maxAge: 15 * 60 * 1000 },
];

// Install event - cache static assets
self.addEventListener('install', (event) => {
  console.log('[SW] Installing...');
  event.waitUntil(
    caches.open(STATIC_CACHE_NAME)
      .then(cache => {
        console.log('[SW] Caching static assets');
        return cache.addAll(STATIC_ASSETS);
      })
      .then(() => self.skipWaiting())
  );
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
  console.log('[SW] Activating...');
  event.waitUntil(
    caches.keys()
      .then(keys => {
        return Promise.all(
          keys
            .filter(key => key !== STATIC_CACHE_NAME && key !== DYNAMIC_CACHE_NAME)
            .map(key => {
              console.log('[SW] Removing old cache:', key);
              return caches.delete(key);
            })
        );
      })
      .then(() => self.clients.claim())
  );
});

// Fetch event - handle requests
self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Skip non-GET requests
  if (request.method !== 'GET') {
    return;
  }

  // Skip chrome-extension and other non-http(s) requests
  if (!url.protocol.startsWith('http')) {
    return;
  }

  // Handle API requests
  if (url.pathname.startsWith('/api/')) {
    event.respondWith(handleApiRequest(request));
    return;
  }

  // Handle static assets
  if (isStaticAsset(url.pathname)) {
    event.respondWith(handleStaticRequest(request));
    return;
  }

  // Handle navigation requests (HTML pages)
  if (request.mode === 'navigate') {
    event.respondWith(handleNavigationRequest(request));
    return;
  }

  // Default: network first
  event.respondWith(networkFirst(request));
});

// Check if URL is a static asset
function isStaticAsset(pathname) {
  return /\.(js|css|png|jpg|jpeg|gif|svg|ico|woff|woff2|ttf|eot|webp|mjs)$/.test(pathname);
}

// Handle API requests with caching strategies
async function handleApiRequest(request) {
  const url = new URL(request.url);

  for (const rule of API_CACHE_RULES) {
    if (rule.pattern.test(url.pathname)) {
      switch (rule.strategy) {
        case 'staleWhileRevalidate':
          return staleWhileRevalidate(request, rule.maxAge);
        case 'networkFirst':
          return networkFirst(request, rule.maxAge);
        case 'cacheFirst':
          return cacheFirst(request);
      }
    }
  }

  // Default: network first
  return networkFirst(request);
}

// Cache-first strategy
async function cacheFirst(request) {
  const cached = await caches.match(request);
  if (cached) {
    return cached;
  }

  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(DYNAMIC_CACHE_NAME);
      cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    return new Response('Offline', { status: 503 });
  }
}

// Network-first strategy
async function networkFirst(request, maxAge) {
  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(DYNAMIC_CACHE_NAME);
      cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    const cached = await caches.match(request);
    if (cached) {
      return cached;
    }
    // Return offline page for navigation requests
    if (request.mode === 'navigate') {
      const offlinePage = await caches.match('/');
      if (offlinePage) {
        return offlinePage;
      }
    }
    return new Response('Offline', { status: 503 });
  }
}

// Stale-while-revalidate strategy
async function staleWhileRevalidate(request, maxAge) {
  const cache = await caches.open(DYNAMIC_CACHE_NAME);
  const cached = await cache.match(request);

  // Fetch in background
  const fetchPromise = fetch(request)
    .then(response => {
      if (response.ok) {
        cache.put(request, response.clone());
      }
      return response;
    })
    .catch(() => null);

  // Return cached immediately if fresh
  if (cached) {
    const age = Date.now() - cached.headers.get('date');
    if (age < maxAge) {
      return cached;
    }
  }

  // Return cached or wait for network
  return cached || fetchPromise;
}

// Handle navigation requests
async function handleNavigationRequest(request) {
  try {
    // Try network first for navigation
    const response = await fetch(request);
    return response;
  } catch (error) {
    // Return cached index.html
    const cached = await caches.match('/');
    if (cached) {
      return cached;
    }
    // Return inline offline page
    return new Response(OFFLINE_PAGE, {
      headers: { 'Content-Type': 'text/html' }
    });
  }
}

// Handle static requests
async function handleStaticRequest(request) {
  const cached = await caches.match(request);
  if (cached) {
    // Return cached and update in background
    fetch(request)
      .then(response => {
        if (response.ok) {
          caches.open(STATIC_CACHE_NAME)
            .then(cache => cache.put(request, response));
        }
      })
      .catch(() => {});
    return cached;
  }

  try {
    const response = await fetch(request);
    if (response.ok) {
      const cache = await caches.open(STATIC_CACHE_NAME);
      cache.put(request, response.clone());
    }
    return response;
  } catch (error) {
    return new Response('Offline', { status: 503 });
  }
}

// Background sync for offline form submissions
self.addEventListener('sync', (event) => {
  if (event.tag === 'sync-messages') {
    event.waitUntil(syncMessages());
  }
});

async function syncMessages() {
  const pending = await getPendingMessages();
  for (const msg of pending) {
    try {
      await fetch('/api/messages', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(msg)
      });
      await removePendingMessage(msg.id);
    } catch (error) {
      console.error('[SW] Sync failed:', error);
    }
  }
}

async function getPendingMessages() {
  // Implementation depends on IndexedDB or localStorage
  return [];
}

async function removePendingMessage(id) {
  // Implementation depends on IndexedDB or localStorage
}

// Push notification handling
self.addEventListener('push', (event) => {
  if (!event.data) return;

  const data = event.data.json();
  const options = {
    body: data.body || 'Bạn có thông báo mới',
    icon: '/icons/icon-192.png',
    badge: '/icons/badge-72.png',
    vibrate: [100, 50, 100],
    data: {
      url: data.url || '/'
    },
    actions: [
      { action: 'open', title: 'Mở' },
      { action: 'dismiss', title: 'Bỏ qua' }
    ]
  };

  event.waitUntil(
    self.registration.showNotification(data.title || 'BizClaw', options)
  );
});

self.addEventListener('notificationclick', (event) => {
  event.notification.close();

  if (event.action === 'dismiss') {
    return;
  }

  const url = event.notification.data.url || '/';
  event.waitUntil(
    clients.matchAll({ type: 'window' })
      .then(clientList => {
        for (const client of clientList) {
          if (client.url === url && 'focus' in client) {
            return client.focus();
          }
        }
        if (clients.openWindow) {
          return clients.openWindow(url);
        }
      })
  );
});

// Offline page HTML
const OFFLINE_PAGE = `
<!DOCTYPE html>
<html lang="vi">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>BizClaw - Offline</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      font-family: 'Inter', system-ui, sans-serif;
      background: #08090d;
      color: #e8ecf4;
      min-height: 100vh;
      display: flex;
      align-items: center;
      justify-content: center;
      text-align: center;
      padding: 20px;
    }
    .container { max-width: 400px; }
    h1 { font-size: 24px; margin-bottom: 16px; }
    p { color: #7c8599; margin-bottom: 24px; }
    button {
      background: linear-gradient(135deg, #6366f1, #8b5cf6);
      color: white;
      border: none;
      padding: 12px 24px;
      border-radius: 8px;
      font-size: 14px;
      font-weight: 600;
      cursor: pointer;
    }
    button:hover { box-shadow: 0 4px 16px rgba(99,102,241,0.35); }
  </style>
</head>
<body>
  <div class="container">
    <h1>📡 Không có kết nối</h1>
    <p>Bạn đang offline. Vui lòng kiểm tra kết nối internet của bạn.</p>
    <button onclick="location.reload()">Thử lại</button>
  </div>
</body>
</html>
`;
