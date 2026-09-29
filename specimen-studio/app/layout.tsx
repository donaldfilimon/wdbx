import type { Metadata } from 'next';
import { Geist, Geist_Mono } from 'next/font/google';
import { THEME_BOOT_SCRIPT } from './shell/theme';
import './studio.css';

const geistSans = Geist({
  variable: '--font-geist-sans',
  subsets: ['latin'],
});

const geistMono = Geist_Mono({
  variable: '--font-geist-mono',
  subsets: ['latin'],
});

export const metadata: Metadata = {
  title: 'WDBX · Specimen Studio',
  icons: { icon: '/favicon.svg' },
  description:
    'Teach patterns, trace votes, and explore the complete WDBX specimen architecture in a private local workspace.',
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased`}
      >
        <script dangerouslySetInnerHTML={{ __html: THEME_BOOT_SCRIPT }} />
        {children}
      </body>
    </html>
  );
}
