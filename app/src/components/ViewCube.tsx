// SPDX-License-Identifier: GPL-3.0-or-later
// Small cube in a corner of the 3D view. It turns with the camera; its faces, edges and corners
// are buttons that turn the view to look from there.
import { useTranslation } from "../i18n/i18n";
import {
  CUBE_VIEWS,
  basisToCss,
  cubeMatrix,
  elementCss,
  elementKind,
  isVerticalEdge,
  type CubeView,
} from "../view-cube";

interface Props {
  /** Camera yaw and pitch in radians, as reported by the native view. */
  yaw: number;
  pitch: number;
  /** A cube view name or "default". */
  onView: (name: CubeView | "default") => void;
}

/** Half the edge length of the cube in CSS pixels. */
const HALF = 36;

export function ViewCube({ yaw, pitch, onView }: Props) {
  const { t } = useTranslation();
  return (
    <div className="view-cube" role="group" aria-label={t("viewCube.label")}>
      <div className="view-cube-stage">
        <div
          className="view-cube-body"
          data-testid="view-cube-body"
          style={{ transform: basisToCss(cubeMatrix(yaw, pitch)) }}
        >
          {CUBE_VIEWS.map((name) => {
            const kind = elementKind(name);
            const vertical = isVerticalEdge(name);
            return (
              <button
                key={name}
                type="button"
                className={`view-cube-${kind}${vertical ? " vertical" : ""}`}
                aria-label={t(`viewCube.${name}`)}
                title={t(`viewCube.${name}`)}
                style={{ transform: elementCss(name, HALF) }}
                onClick={() => onView(name)}
              >
                {kind === "face" ? t(`viewCube.short.${name}`) : null}
              </button>
            );
          })}
        </div>
      </div>
      <button
        type="button"
        className="view-cube-home"
        aria-label={t("viewCube.default")}
        title={t("viewCube.default")}
        onClick={() => onView("default")}
      >
        {t("viewCube.defaultShort")}
      </button>
    </div>
  );
}
