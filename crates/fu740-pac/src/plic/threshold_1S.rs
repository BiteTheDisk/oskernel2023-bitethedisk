#[doc = "Register `threshold_1S` reader"]
pub struct R(crate::R<THRESHOLD_1S_SPEC>);
impl core::ops::Deref for R {
    type Target = crate::R<THRESHOLD_1S_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<crate::R<THRESHOLD_1S_SPEC>> for R {
    #[inline(always)]
    fn from(reader: crate::R<THRESHOLD_1S_SPEC>) -> Self {
        R(reader)
    }
}
#[doc = "Register `threshold_1S` writer"]
pub struct W(crate::W<THRESHOLD_1S_SPEC>);
impl core::ops::Deref for W {
    type Target = crate::W<THRESHOLD_1S_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl core::ops::DerefMut for W {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<crate::W<THRESHOLD_1S_SPEC>> for W {
    #[inline(always)]
    fn from(writer: crate::W<THRESHOLD_1S_SPEC>) -> Self {
        W(writer)
    }
}
impl W {
    #[doc = "Writes raw bits to the register."]
    #[inline(always)]
    pub unsafe fn bits(&mut self, bits: u32) -> &mut Self {
        self.0.bits(bits);
        self
    }
}
#[doc = "PRIORITY THRESHOLD Register for hart 1 S-Mode\n\nThis register you can [`read`](crate::generic::Reg::read), [`write_with_zero`](crate::generic::Reg::write_with_zero), [`modify`](crate::generic::Reg::modify). See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [threshold_1S](index.html) module"]
pub struct THRESHOLD_1S_SPEC;
impl crate::RegisterSpec for THRESHOLD_1S_SPEC {
    type Ux = u32;
}
#[doc = "`read()` method returns [threshold_1S::R](R) reader structure"]
impl crate::Readable for THRESHOLD_1S_SPEC {
    type Reader = R;
}
#[doc = "`write(|w| ..)` method takes [threshold_1S::W](W) writer structure"]
impl crate::Writable for THRESHOLD_1S_SPEC {
    type Writer = W;
}
