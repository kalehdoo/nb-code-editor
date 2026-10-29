const { defineConfig, devices } = require('@playwright/test');
module.exports = defineConfig({
  testDir: './tests',
  use: { baseURL: 'http://127.0.0.1:8080' },
  webServer: [
    { command: 'cargo run -p editor-demo-server', url: 'http://127.0.0.1:39181/health', timeout: 180000, reuseExistingServer: !process.env.CI },
    { command: 'trunk serve --address 127.0.0.1 --port 8080', cwd: 'examples/demo', url: 'http://127.0.0.1:8080', timeout: 180000, reuseExistingServer: !process.env.CI }
  ],
  projects: [
    { name: 'chromium', use: { ...devices['Desktop Chrome'] } },
    { name: 'firefox', use: { ...devices['Desktop Firefox'] } },
    { name: 'webkit', use: { ...devices['Desktop Safari'] } }
  ]
});
