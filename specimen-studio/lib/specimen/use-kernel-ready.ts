import { useSyncExternalStore } from 'react';
import { kernelReady, subscribeKernel } from './kernel';

/** True once the WASM kernel has loaded; re-renders when it does. */
export const useKernelReady = () =>
  useSyncExternalStore(subscribeKernel, kernelReady, () => false);
