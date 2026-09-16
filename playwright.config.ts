import { defineConfig } from '@playwright/test'
process.env.NO_PROXY = [process.env.NO_PROXY, 'localhost', '127.0.0.1'].filter(Boolean).join(',')
export default defineConfig({ testDir:'tests/e2e', timeout:30000, use:{ baseURL:'http://localhost:1420', viewport:{width:960,height:640}, trace:'retain-on-failure', channel:'chromium' }, webServer:{ command:'npm run dev', url:'http://localhost:1420', reuseExistingServer:true }, workers:1 })
