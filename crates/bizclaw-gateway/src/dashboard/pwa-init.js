// BizClaw PWA Initialization
// Registers service worker and manages PWA features

export function initPWA() {
  // Register service worker
  if ('serviceWorker' in navigator) {
    window.addEventListener('load', () => {
      navigator.serviceWorker.register('/sw.js')
        .then(registration => {
          console.log('[PWA] Service Worker registered:', registration.scope);
          
          // Check for updates
          registration.addEventListener('updatefound', () => {
            const newWorker = registration.installing;
            newWorker.addEventListener('statechange', () => {
              if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
                // New content available
                console.log('[PWA] New content available');
                window.dispatchEvent(new CustomEvent('pwa-update-available'));
              }
            });
          });
        })
        .catch(error => {
          console.error('[PWA] Service Worker registration failed:', error);
        });
    });

    // Handle controller change (new SW activated)
    navigator.serviceWorker.addEventListener('controllerchange', () => {
      console.log('[PWA] Controller changed, reloading...');
      window.location.reload();
    });
  }

  // Register PWA manifest
  if ('standalone' in window.navigator) {
    document.documentElement.setAttribute('data-pwa', 'installed');
  }

  // Handle install prompt
  let deferredPrompt;
  window.addEventListener('beforeinstallprompt', (e) => {
    e.preventDefault();
    deferredPrompt = e;
    window.dispatchEvent(new CustomEvent('pwa-install-ready', { detail: deferredPrompt }));
  });

  window.addEventListener('appinstalled', () => {
    console.log('[PWA] App installed');
    deferredPrompt = null;
    window.dispatchEvent(new CustomEvent('pwa-installed'));
  });

  // Handle standalone mode
  if (window.matchMedia('(display-mode: standalone)').matches) {
    document.documentElement.setAttribute('data-standalone', 'true');
    console.log('[PWA] Running in standalone mode');
  }

  // Handle online/offline
  window.addEventListener('online', () => {
    document.documentElement.removeAttribute('data-offline');
    console.log('[PWA] Back online');
  });

  window.addEventListener('offline', () => {
    document.documentElement.setAttribute('data-offline', 'true');
    console.log('[PWA] Gone offline');
  });

  // Register Web Push (if supported)
  if ('Notification' in window && Notification.permission === 'default') {
    // Don't auto-request, let user decide
    console.log('[PWA] Push notifications available');
  }
}

// Prompt user to install PWA
export async function promptInstall() {
  const event = await new Promise(resolve => {
    window.addEventListener('pwa-install-ready', resolve, { once: true });
    setTimeout(resolve, 5000); // Timeout after 5s
  });

  if (!event || !event.detail) {
    return false;
  }

  event.detail.prompt();
  const { outcome } = await event.detail.userChoice;
  console.log('[PWA] Install prompt:', outcome);
  return outcome === 'accepted';
}

// Request push notification permission
export async function requestNotificationPermission() {
  if (!('Notification' in window)) {
    return false;
  }

  if (Notification.permission === 'granted') {
    return true;
  }

  if (Notification.permission !== 'denied') {
    const permission = await Notification.requestPermission();
    return permission === 'granted';
  }

  return false;
}

// Subscribe to push notifications
export async function subscribePush() {
  if (!('serviceWorker' in navigator) || !('PushManager' in window)) {
    return null;
  }

  const registration = await navigator.serviceWorker.ready;
  const subscription = await registration.pushManager.subscribe({
    userVisibleOnly: true,
    applicationServerKey: urlBase64ToUint8Array(VAPID_PUBLIC_KEY)
  });

  return subscription;
}

// Helper: Convert VAPID key
function urlBase64ToUint8Array(base64String) {
  const padding = '='.repeat((4 - base64String.length % 4) % 4);
  const base64 = (base64String + padding)
    .replace(/-/g, '+')
    .replace(/_/g, '/');
  const rawData = window.atob(base64);
  const outputArray = new Uint8Array(rawData.length);
  for (let i = 0; i < rawData.length; ++i) {
    outputArray[i] = rawData.charCodeAt(i);
  }
  return outputArray;
}

// VAPID Public Key (placeholder - should be configured per deployment)
const VAPID_PUBLIC_KEY = 'BEl62iUYgUivxIkv69yViEuiqL7aMGjP6oNbTX7NoRVXEn6aBfwzOD6HCjbgfZfR';

// Auto-init on load
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', initPWA);
} else {
  initPWA();
}
