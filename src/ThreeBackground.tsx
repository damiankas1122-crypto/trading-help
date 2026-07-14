import { useEffect, useRef } from "react";
import * as THREE from "three";


export default function ThreeBackground() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const scene = new THREE.Scene();
    scene.fog = new THREE.FogExp2(0x05050a, 0.035);

    const camera = new THREE.PerspectiveCamera(
      60,
      window.innerWidth / window.innerHeight,
      0.1,
      1000
    );
    camera.position.set(0, 6, 16);
    camera.lookAt(0, -1, 0);

    const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
    renderer.setSize(window.innerWidth, window.innerHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

    const gridSize = 40;
    const segments = 48;
    const geometry = new THREE.PlaneGeometry(gridSize, gridSize, segments, segments);
    geometry.rotateX(-Math.PI / 2);
    const material = new THREE.MeshBasicMaterial({
      color: 0x0891b2,
      wireframe: true,
      transparent: true,
      opacity: 0.3,
    });
    const mesh = new THREE.Mesh(geometry, material);
    mesh.position.y = -2;
    scene.add(mesh);

    const pos = geometry.attributes.position;
    const basePositions = pos.array.slice();

    let frameId: number;
    const animate = (t: number) => {
      const time = t * 0.00045;
      for (let i = 0; i < pos.count; i++) {
        const x = basePositions[i * 3];
        const z = basePositions[i * 3 + 2];
        const wave = Math.sin(x * 0.35 + time) * 0.6 + Math.cos(z * 0.35 + time * 1.3) * 0.6;
        pos.array[i * 3 + 1] = wave;
      }
      pos.needsUpdate = true;
      mesh.rotation.y = Math.sin(time * 0.15) * 0.05;
      renderer.render(scene, camera);
      frameId = requestAnimationFrame(animate);
    };
    frameId = requestAnimationFrame(animate);

    const handleResize = () => {
      camera.aspect = window.innerWidth / window.innerHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(window.innerWidth, window.innerHeight);
    };
    window.addEventListener("resize", handleResize);

    return () => {
      cancelAnimationFrame(frameId);
      window.removeEventListener("resize", handleResize);
      geometry.dispose();
      material.dispose();
      renderer.dispose();
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      style={{ position: "fixed", inset: 0, zIndex: 0, pointerEvents: "none" }}
    />
  );
}
