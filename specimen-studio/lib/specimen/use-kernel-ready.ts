import { useSyncExternalStore } from 'react';
import { kernelStatus, subscribeKernel, type LoadStatus } from './kernel';

/** The kernel's load status; re-renders when it loads or fails. */
export const useKernelStatus = (): LoadStatus =>
  useSyncExternalStore(subscribeKernel, kernelStatus, () => 'loading');
