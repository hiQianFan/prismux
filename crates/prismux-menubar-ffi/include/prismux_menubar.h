#ifndef OMX_MENUBAR_H
#define OMX_MENUBAR_H

#ifdef __cplusplus
extern "C" {
#endif

char *prismux_menubar_call(const char *request_json);
void prismux_menubar_free(char *value);

#ifdef __cplusplus
}
#endif

#endif

