import type { NextConfig } from 'next';
import path from 'path';

const nextConfig: NextConfig = {
  output: 'export',
  distDir: 'dist',
  poweredByHeader: false,
  productionBrowserSourceMaps: false,
  ...(process.env.NODE_ENV === 'production'
    ? {
        compiler: {
          removeConsole: { exclude: ['error', 'warn'] },
        },
      }
    : {}),
  images: { unoptimized: true },
  turbopack: {
    root: path.resolve(__dirname),
  },
};

export default nextConfig;
