# LiteRT-LM reaches its Kotlin surface from JNI, so members must survive shrinking.
-keep class com.google.ai.edge.litertlm.** { *; }
-keepclasseswithmembernames class * { native <methods>; }

# kotlinx.serialization generated serializers.
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.**
-keepclassmembers class vn.bizclaw.agent.** {
    *** Companion;
    kotlinx.serialization.KSerializer serializer(...);
}
