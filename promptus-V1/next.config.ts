import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Standalone output for minimal Docker images.
  output: "standalone",
  images: {
    remotePatterns: [
      { protocol: "https", hostname: "cdn.pixabay.com" },
    ],
  },
};

export default nextConfig;
