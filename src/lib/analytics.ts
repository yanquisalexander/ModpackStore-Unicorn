import { init } from '@aptabase/web';

export const initAnalytics = () => {
    init('A-US-3025010252', {
        isDebug: import.meta.env.MODE === 'development',
    })
};