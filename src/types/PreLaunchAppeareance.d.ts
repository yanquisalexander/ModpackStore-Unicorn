export interface PreLaunchAppearance {
    title?: string;
    description?: string;
    logo?: Logo;
    playButton?: PlayButton;
    background?: Background;
    audio?: Audio;
    news?: News;
    footerStyle?: FooterStyle;
    footerText?: string;
}

export interface Audio {
    url?: string;
    volume?: number;
}

export interface Background {
    imageUrl?: string;
    videoUrl?: null;
}

export interface Logo {
    url?: string;
    height?: string;
    position?: LogoPosition;
    fadeInDuration?: string;
    fadeInDelay?: string;
}

export interface LogoPosition {
    top?: string;
    left?: string;
    right?: string;
    bottom?: string;
    transform?: string;
}

export interface News {
    position?: NewsPosition;
    style?: Style;
    entries?: Entry[];
}

export interface Entry {
    title?: string;
    content?: string;
}

export interface NewsPosition {
    top?: string;
    right?: string;
}

export interface Style {
    background?: string;
    color?: string;
    borderRadius?: string;
    padding?: string;
    width?: string;
    fontSize?: string;
}

export interface PlayButton {
    text?: string;
    fontFamily?: string;
    backgroundColor?: string;
    hoverColor?: string;
    position?: PlayButtonPosition;
    textColor?: string;
    borderColor?: string;
    fadeInDuration?: string;
    fadeInDelay?: string;
    position?: PlayButtonPosition;
}

export interface PlayButtonPosition {
    bottom?: string;
    left?: string;
    right?: string;
    top?: string;
    transform?: string;
}

export interface FooterStyle {
    background?: string;
    color?: string;
    borderRadius?: string;
    padding?: string;
    width?: string;
    fontSize?: string;
}
