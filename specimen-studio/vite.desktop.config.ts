import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/postcss';
import { resolve } from 'node:path';
export default defineConfig({root:'desktop',publicDir:'../public',base:'./',plugins:[react()],resolve:{alias:{'@':resolve(import.meta.dirname,'.')}},css:{postcss:{plugins:[tailwindcss()]}},server:{host:'localhost',port:1420,strictPort:true},build:{outDir:'../desktop-dist',emptyOutDir:true}});
