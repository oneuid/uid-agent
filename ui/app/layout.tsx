import type { Metadata } from 'next';
import './globals.css';
import { Toaster } from 'sonner';
import { I18nProvider } from '../lib/i18n';

export const metadata: Metadata = {
  title: 'UID Agent | Sovereign Identity Node',
  description: 'Endpoint hardware security and PKCS#11 smartcard bridge',
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" dir="ltr" className="dark">
      <body className="bg-slate-950 text-slate-100 antialiased min-h-dvh">
        <I18nProvider>
          {children}
          <Toaster
            position="bottom-right"
            theme="dark"
            toastOptions={{
              style: {
                background: '#0b1120',
                border: '1px solid rgba(51, 65, 85, 0.8)',
                color: '#f8fafc',
                fontSize: '12px',
                fontFamily: 'var(--font-inter)'
              }
            }}
          />
        </I18nProvider>
      </body>
    </html>
  );
}
