export type ReferenceView = {
  label: string;
  url: string;
};

export type Reference = {
  id: string;
  group: "surface" | "weather" | "cloud";
  name: string;
  owner: string;
  role: string;
  technique: string;
  boundary: string;
  commit: string;
  license: string;
  repository: string;
  runtime: "local" | "official";
  views: ReferenceView[];
};

export const groupCopy = {
  surface: {
    eyebrow: "SURFACE / 01",
    title: "玻璃材质",
    summary: "三种实现完全隔离。看边缘折射、高光、模糊位置与卡片曲率，不看对照页自身样式。",
  },
  weather: {
    eyebrow: "WEATHER / 02",
    title: "天气粒子",
    summary: "对照完整天气演示与 Three.js 官方计算粒子，确认雨、雪、雾分别属于哪条渲染路线。",
  },
  cloud: {
    eyebrow: "CLOUD / 03",
    title: "云层体积",
    summary: "把广告牌云、示例体积云和计算体积云分开看，禁止把三者混称为体积云。",
  },
} as const;

export const references: Reference[] = [
  {
    id: "glace",
    group: "surface",
    name: "Glacé",
    owner: "seangeng",
    role: "毛玻璃 UI 基线",
    technique: "尺寸匹配 SVG 位移图、轻模糊、色差边缘、颗粒与掠射高光",
    boundary: "它是 React UI 表面，不是 WebGL 液态透镜。",
    commit: "c8f5a363ab2b",
    license: "MIT",
    repository: "https://github.com/seangeng/glace",
    runtime: "local",
    views: [
      { label: "Primitives", url: "http://127.0.0.1:4191/primitives" },
      { label: "Panels", url: "http://127.0.0.1:4191/panels" },
      { label: "Buttons", url: "http://127.0.0.1:4191/buttons" },
    ],
  },
  {
    id: "shuding",
    group: "surface",
    name: "Liquid Glass",
    owner: "shuding",
    role: "SVG 液态透镜基线",
    technique: "CPU 位移图、SVG feDisplacementMap、圆角 SDF 与可拖动透镜",
    boundary: "它是一个 300 × 200 的实验透镜，不是通用卡片组件库。",
    commit: "a2d2e847f793",
    license: "MIT",
    repository: "https://github.com/shuding/liquid-glass",
    runtime: "local",
    views: [{ label: "Original script", url: "/references/shuding.html" }],
  },
  {
    id: "ybouane",
    group: "surface",
    name: "LiquidGlass",
    owner: "ybouane",
    role: "WebGL HTML 卡片基线",
    technique: "DOM 捕获、多通道模糊、弧面高度场、折射、Fresnel 与高光",
    boundary: "直接子元素和 DOM 捕获存在结构、跨域与性能约束。",
    commit: "5ebda520bebd",
    license: "MIT declared · root LICENSE missing",
    repository: "https://github.com/ybouane/liquidglass",
    runtime: "local",
    views: [{ label: "Official demo", url: "http://127.0.0.1:4192/" }],
  },
  {
    id: "react-weather",
    group: "weather",
    name: "React Weather Effects",
    owner: "rauschermate",
    role: "完整天气观感",
    technique: "雨滴折射、水图合成、雪粒子、雾平面与天气预设",
    boundary: "作为观感与参数来源，不直接复制其页面生命周期代码。",
    commit: "8326628e18cf",
    license: "MIT",
    repository: "https://github.com/rauschermate/react-weather-effects",
    runtime: "local",
    views: [
      { label: "Rain", url: "http://127.0.0.1:4193/rain" },
      { label: "Snow", url: "http://127.0.0.1:4193/snow" },
      { label: "Fog", url: "http://127.0.0.1:4193/fog" },
    ],
  },
  {
    id: "three-rain",
    group: "weather",
    name: "Compute Rain / Snow",
    owner: "three.js examples",
    role: "官方 WebGPU 粒子基线",
    technique: "计算缓冲、实例化粒子、碰撞高度图、涟漪与积雪",
    boundary: "这是 3D 技术样例，不代表 MindScape 的 UI 风格。",
    commit: "dev · 09860b8ff9f8",
    license: "MIT",
    repository: "https://github.com/mrdoob/three.js/tree/dev/examples",
    runtime: "official",
    views: [
      { label: "Rain", url: "https://threejs.org/examples/webgpu_compute_particles_rain.html" },
      { label: "Snow", url: "https://threejs.org/examples/webgpu_compute_particles_snow.html" },
    ],
  },
  {
    id: "drei-cloud",
    group: "cloud",
    name: "Cloud",
    owner: "pmndrs / drei",
    role: "广告牌云基线",
    technique: "相机朝向的纹理平面、实例化、体积分布与淡出",
    boundary: "它看起来蓬松，但不是光线步进体积云。",
    commit: "ffa15b956e32",
    license: "MIT",
    repository: "https://github.com/pmndrs/drei/blob/master/src/core/Cloud.tsx",
    runtime: "official",
    views: [{ label: "Storybook canvas", url: "https://drei.pmnd.rs/iframe.html?viewMode=story&id=staging-cloud--cloud-st" }],
  },
  {
    id: "three-volume",
    group: "cloud",
    name: "Volume Cloud",
    owner: "three.js examples",
    role: "封闭体积示例",
    technique: "3D Perlin 纹理与 RaymarchingBox 光线步进",
    boundary: "是局部体积盒示例，不是地平线天气系统。",
    commit: "dev · 09860b8ff9f8",
    license: "MIT",
    repository: "https://github.com/mrdoob/three.js/blob/dev/examples/webgpu_volume_cloud.html",
    runtime: "official",
    views: [{ label: "Official example", url: "https://threejs.org/examples/webgpu_volume_cloud.html" }],
  },
  {
    id: "josh-clouds",
    group: "cloud",
    name: "Realtime Clouds",
    owner: "joshbrew",
    role: "计算体积云基线",
    technique: "WebGPU compute、体积纹理、太阳散射、时间重投影与低分辨率合成",
    boundary: "高性能档专用，必须保留 worker、分辨率分级和失败回退。",
    commit: "3863234f739d",
    license: "MIT",
    repository: "https://github.com/joshbrew/webgpu_realtime_clouds",
    runtime: "local",
    views: [{ label: "Cloud playground", url: "http://127.0.0.1:4194/" }],
  },
];
