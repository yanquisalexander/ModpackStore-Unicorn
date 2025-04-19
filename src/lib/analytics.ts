import { init, trackEvent } from '@aptabase/web';

export const initAnalytics = () => {
    init('A-US-3025010252', {
        isDebug: import.meta.env.MODE === 'development',
    })
};

export const trackSectionView = (section: string) => {
    trackEvent('section_view', {
        name: `Section View: ${section}`,
        section,
        timestamp: new Date().toISOString(),
    })
}