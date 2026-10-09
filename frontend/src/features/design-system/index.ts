/**
 * The design system gallery (dev builds only): the `/design` route loads it
 * behind `import.meta.env.DEV`, so release bundles never include it.
 */
export { DesignScreen } from './design-screen';
