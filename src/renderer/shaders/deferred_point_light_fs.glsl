#version 330 core

uniform sampler2D depthTexture;
uniform sampler2D colorTexture;
uniform sampler2D normalTexture;
uniform sampler2D materialTexture;
uniform samplerCube pointShadowTexture;

uniform vec3 lightPos;
uniform float lightRadius;
uniform vec4 lightColor;
uniform mat4 invViewProj;
uniform vec3 cameraPosition;
uniform bool softShadows;
uniform bool shadowsEnabled;
uniform float shadowBias;
uniform float lightIntensity;

in vec2 texCoord;
out vec4 FragColor;

void main()
{
    vec3 material = texture(materialTexture, texCoord).rgb;

    vec3 fragmentPosition = S_UnProject(vec3(texCoord, texture(depthTexture, texCoord).r), invViewProj);
    vec3 fragmentToLight = lightPos - fragmentPosition;
    float distance = length(fragmentToLight);

    TPBRContext ctx;
    ctx.albedo = texture(colorTexture, texCoord).rgb;
    ctx.fragmentToLight = fragmentToLight / distance;
    ctx.fragmentNormal = normalize(texture(normalTexture, texCoord).xyz * 2.0 - 1.0);
    ctx.lightColor = lightColor.rgb;
    ctx.metallic = material.x;
    ctx.roughness = material.y;
    ctx.viewVector = normalize(cameraPosition - fragmentPosition);

    vec3 lighting = S_PBR_CalculateLight(ctx);

    float distanceAttenuation = S_LightDistanceAttenuation(distance, lightRadius);

    float shadow = S_PointShadow(
        shadowsEnabled, softShadows, distance, shadowBias, ctx.fragmentToLight, pointShadowTexture);

    FragColor = vec4(lightIntensity * distanceAttenuation * shadow * lighting, 1.0);
}
