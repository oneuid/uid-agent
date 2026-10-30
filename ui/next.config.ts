import type { NextConfig } from 'next';
import path from 'path';

const nextConfig: NextConfig = {
  output: 'export',
  distDir: 'dist',
  poweredByHeader: false,
  productionBrowserSourceMaps: false,
  compiler: {
    removeConsole:
      process.env.NODE_ENV === 'production'
        ? { exclude: ['error', 'warn'] }
        : false,
  },
  images: { unoptimized: true },
  turbopack: {
    root: path.resolve(__dirname),
  },
};

export default nextConfig;
