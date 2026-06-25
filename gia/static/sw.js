const CACHE_NAME = 'gia-pwa-v1';

// instalación
self.addEventListener('install', (event) => {
    self.skipWaiting();
});

// activación
self.addEventListener('activate', (event) => {
    event.waitUntil(clients.claim());
});

// Interceptor de peticiones
self.addEventListener('fetch', (event) => {
    // Para que la PWA sea instalable, debe tener un fetch handler.
    // Simplemente dejamos pasar la petición a la red.
    event.respondWith(fetch(event.request));
});